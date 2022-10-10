// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package videoingress

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/rtmpingress"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// TODO: move to server config
const streamUpdateInterval = time.Minute

func newIngressService(
	ctx context.Context,
	logger *zap.Logger,
	store dao.Store,
	transfer transfer.Control,
	network network.Control,
	directory directory.Control,
) *ingressService {
	return &ingressService{
		ctx:        ctx,
		logger:     logger,
		store:      store,
		transfer:   transfer,
		network:    network,
		directory:  directory,
		transcoder: rtmpingress.NewTranscoder(logger),
		streams:    map[uint64]*ingressStream{},
	}
}

type ingressService struct {
	ctx        context.Context
	logger     *zap.Logger
	store      dao.Store
	transfer   transfer.Control
	network    network.Control
	directory  directory.Control
	transcoder *rtmpingress.Transcoder

	lock    sync.Mutex
	streams map[uint64]*ingressStream
}

func (s *ingressService) UpdateChannel(channel *videov1.VideoChannel) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if stream, ok := s.streams[channel.Id]; ok {
		stream.UpdateChannel(channel)
	}
}

func (s *ingressService) RemoveChannel(id uint64) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if stream, ok := s.streams[id]; ok {
		stream.Close()
	}
}

func (s *ingressService) HandleStream(a *rtmpingress.StreamAddr, c *rtmpingress.Conn) {
	defer c.Close()

	stream, err := newIngressStream(
		s.ctx,
		s.logger,
		s.store,
		s.transfer,
		s.network,
		s.directory,
		a,
		c,
	)
	if err != nil {
		s.logger.Warn(
			"setting up stream failed",
			zap.String("key", a.Key),
			zap.Error(err),
		)
		return
	}
	defer stream.Close()

	go func() {
		err := s.transcoder.Transcode(
			c.Context(),
			a.URI,
			a.Key,
			"source",
			stream.w,
		)
		if err != nil {
			stream.logger.Debug(
				"transcoder finished",
				zap.Error(err),
			)
		}
	}()

	stream.logger.Info("rtmp stream opened")

	s.lock.Lock()
	s.streams[stream.channelID] = stream
	s.lock.Unlock()

	<-c.CloseNotify()
	stream.logger.Debug("rtmp stream closed")

	s.lock.Lock()
	delete(s.streams, stream.channelID)
	s.lock.Unlock()
}

func (s *ingressService) HandlePassthruStream(a *rtmpingress.StreamAddr, c *rtmpingress.Conn) (ioutil.WriteFlusher, error) {
	stream, err := newIngressStream(
		s.ctx,
		s.logger,
		s.store,
		s.transfer,
		s.network,
		s.directory,
		a,
		c,
	)
	if err != nil {
		s.logger.Debug(
			"setting up stream failed",
			zap.String("key", a.Key),
			zap.Error(err),
		)
		return nil, err
	}

	stream.logger.Debug("rtmp stream opened")

	s.lock.Lock()
	s.streams[stream.channelID] = stream
	s.lock.Unlock()

	go func() {
		<-c.CloseNotify()
		stream.logger.Debug("rtmp stream closed")

		s.lock.Lock()
		delete(s.streams, stream.channelID)
		s.lock.Unlock()

		stream.Close()
	}()

	return stream.w, nil
}

func newIngressStream(
	ctx context.Context,
	logger *zap.Logger,
	store dao.Store,
	transfer transfer.Control,
	network network.Control,
	directory directory.Control,
	addr *rtmpingress.StreamAddr,
	conn io.Closer,
) (s *ingressStream, err error) {
	channel, err := dao.GetVideoChannelByStreamKey(store, addr.Key)
	if err != nil {
		return nil, fmt.Errorf("getting channel: %w", err)
	}

	ctx, cancel := context.WithCancel(ctx)

	s = &ingressStream{
		logger:    logger.With(zap.Uint64("channel", channel.Id)),
		store:     store,
		transfer:  transfer,
		network:   network,
		directory: directory,

		ctx:       ctx,
		cancelCtx: cancel,

		startTime:      timeutil.Now(),
		channelID:      channel.Id,
		channel:        syncutil.NewPointer(channel),
		channelUpdates: make(chan struct{}, 1),
		conn:           conn,
	}

	s.channelUpdates <- struct{}{}

	mu := dao.NewMutex(logger, store, channel.Id)
	if _, err := mu.TryLock(ctx); err != nil {
		return nil, fmt.Errorf("acquiring stream lock: %w", err)
	}

	s.swarm, s.w, err = s.openWriter()
	if err != nil {
		s.Close()
		return nil, fmt.Errorf("opening output stream: %w", err)
	}

	s.transferID = s.transfer.Add(s.swarm, []byte{})
	s.transfer.Publish(s.transferID, s.channelNetworkKey())

	snippet := proto.Clone(channel.DirectoryListingSnippet).(*networkv1directory.ListingSnippet)
	if err := dao.SignMessage(snippet, channel.Key); err != nil {
		return nil, fmt.Errorf("signing snippet: %w", err)
	}
	s.directory.PushSnippet(s.swarm.ID(), snippet)
	go s.syncDirectorySnippet()

	go func() {
		id, err := s.publishDirectoryListing()
		if err != nil {
			s.logger.Debug("publishing stream to directory failed", zap.Error(err))
		}
		s.directoryID = id
	}()

	return s, nil
}

type ingressStream struct {
	logger    *zap.Logger
	store     dao.Store
	transfer  transfer.Control
	network   network.Control
	directory directory.Control

	ctx       context.Context
	cancelCtx context.CancelFunc
	closeOnce sync.Once

	startTime      timeutil.Time
	channelID      uint64
	channel        atomic.Pointer[videov1.VideoChannel]
	channelUpdates chan struct{}
	directoryID    uint64
	conn           io.Closer

	swarm      *ppspp.Swarm
	transferID transfer.ID
	w          *ioutil.WriteFlushSampler
}

func (s *ingressStream) Close() {
	s.closeOnce.Do(func() {
		s.cancelCtx()
		s.conn.Close()

		s.transfer.Remove(s.transferID)
		s.unpublishDirectoryListing()
		s.directory.DeleteSnippet(s.swarm.ID())
	})
}

func (s *ingressStream) UpdateChannel(channel *videov1.VideoChannel) {
	s.channel.Swap(channel)

	select {
	case s.channelUpdates <- struct{}{}:
	case <-s.ctx.Done():
	default:
	}
}

func (s *ingressStream) openWriter() (*ppspp.Swarm, *ioutil.WriteFlushSampler, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:          1024,
			ChunksPerSignature: 32,
			StreamCount:        16,
			LiveWindow:         16 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
				MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
				LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
			},
		},
		Key: s.channel.Load().Key,
	})
	if err != nil {
		return nil, nil, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.DefaultSize)
	if err != nil {
		return nil, nil, err
	}

	return w.Swarm(), ioutil.NewWriteFlushSampler(cw), nil
}

func (s *ingressStream) channelNetworkKey() []byte {
	switch o := s.channel.Load().Owner.(type) {
	case *videov1.VideoChannel_Local_:
		return o.Local.NetworkKey
	case *videov1.VideoChannel_LocalShare_:
		return dao.CertificateRoot(o.LocalShare.Certificate).Key
	default:
		panic("unsupported channel")
	}
}

func (s *ingressStream) channelCreatorCert() (*certificate.Certificate, error) {
	switch o := s.channel.Load().Owner.(type) {
	case *videov1.VideoChannel_Local_:
		cert, ok := s.network.Certificate(o.Local.NetworkKey)
		if !ok {
			return nil, errors.New("network certificate not found")
		}
		return cert, nil
	case *videov1.VideoChannel_LocalShare_:
		return o.LocalShare.Certificate, nil
	default:
		return nil, errors.New("unsupported channel")
	}
}

func (s *ingressStream) publishDirectoryListing() (uint64, error) {
	listing := &networkv1directory.Listing{
		Content: &networkv1directory.Listing_Media_{
			Media: &networkv1directory.Listing_Media{
				MimeType: rtmpingress.TranscoderMimeType,
				SwarmUri: s.swarm.URI().String(),
			},
		},
	}

	return s.directory.Publish(context.Background(), listing, s.channelNetworkKey())
}

func (s *ingressStream) unpublishDirectoryListing() error {
	return s.directory.Unpublish(context.Background(), s.directoryID, s.channelNetworkKey())
}

func (s *ingressStream) syncDirectorySnippet() {
	snippet := &networkv1directory.ListingSnippet{
		StartTime: time.Now().Unix(),
		Live:      true,
	}

	var segment bytes.Buffer

	var thumbnailer rtmpingress.Thumbnailer
	defer thumbnailer.Close()

	t := timeutil.DefaultTickEmitter.Ticker(streamUpdateInterval)
	for {
		select {
		case <-s.channelUpdates:
			channelSnippet := s.channel.Load().DirectoryListingSnippet
			snippet.Title = channelSnippet.Title
			snippet.Description = channelSnippet.Description
			snippet.Tags = channelSnippet.Tags
			snippet.Category = channelSnippet.Category
			snippet.ChannelName = channelSnippet.ChannelName
			snippet.IsMature = channelSnippet.IsMature
			snippet.ChannelLogo = channelSnippet.ChannelLogo
		case <-t.C:
		case <-s.ctx.Done():
			return
		}

		segment.Reset()
		if err := s.w.Sample(&segment); err != nil {
			s.logger.Debug("sampling stream failed", zap.Error(err))
			continue
		}

		if segment.Len() < 2 {
			s.logger.Debug("stream sample too short")
			continue
		}

		img, err := thumbnailer.GetImageFromMp4(segment.Bytes()[2:])
		if err != nil {
			s.logger.Debug("generating stream thumbnail failed", zap.Error(err))
			continue
		}

		snippet.Thumbnail = &networkv1directory.ListingSnippetImage{
			SourceOneof: &networkv1directory.ListingSnippetImage_Image{
				Image: &image.Image{
					Type: image.ImageType_IMAGE_TYPE_JPEG,
					Data: img,
				},
			},
		}

		if err := dao.SignMessage(snippet, s.channel.Load().Key); err != nil {
			s.logger.Debug("signing listing snippet failed", zap.Error(err))
			continue
		}
		s.directory.PushSnippet(s.swarm.ID(), snippet)
	}
}
