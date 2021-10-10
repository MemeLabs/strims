//go:build !js
// +build !js

package videoingress

import (
	"context"
	"errors"
	"fmt"
	"io"
	"strconv"
	"sync"
	"time"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"go.uber.org/zap"
)

const streamLockTimeout = time.Minute * 2
const streamUpdateInterval = time.Minute

func newIngressService(
	logger *zap.Logger,
	store *dao.ProfileStore,
	transfer control.TransferControl,
	network control.NetworkControl,
	directory control.DirectoryControl,
) *ingressService {
	return &ingressService{
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
	logger     *zap.Logger
	store      *dao.ProfileStore
	transfer   control.TransferControl
	network    control.NetworkControl
	directory  control.DirectoryControl
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
		s.logger,
		s.store,
		s.transfer,
		s.network,
		s.directory,
		a,
		c,
	)
	if err != nil {
		s.logger.Info(
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
			s.logger.Debug(
				"transcoder finished",
				zap.Uint64("id", stream.channel.Id),
				zap.Error(err),
			)
		}
	}()

	s.logger.Info("rtmp stream opened", zap.Uint64("id", stream.channel.Id))

	s.lock.Lock()
	s.streams[stream.channel.Id] = stream
	s.lock.Unlock()

	<-c.CloseNotify()
	s.logger.Debug("rtmp stream closed", zap.Uint64("id", stream.channel.Id))

	s.lock.Lock()
	delete(s.streams, stream.channel.Id)
	s.lock.Unlock()
}

func (s *ingressService) HandlePassthruStream(a *rtmpingress.StreamAddr, c *rtmpingress.Conn) (ioutil.WriteFlusher, error) {
	stream, err := newIngressStream(
		s.logger,
		s.store,
		s.transfer,
		s.network,
		s.directory,
		a,
		c,
	)
	if err != nil {
		s.logger.Info(
			"setting up stream failed",
			zap.String("key", a.Key),
			zap.Error(err),
		)
		return nil, err
	}

	s.logger.Info("rtmp stream opened", zap.Uint64("id", stream.channel.Id))

	s.lock.Lock()
	s.streams[stream.channel.Id] = stream
	s.lock.Unlock()

	go func() {
		<-c.CloseNotify()
		s.logger.Debug("rtmp stream closed", zap.Uint64("id", stream.channel.Id))

		s.lock.Lock()
		delete(s.streams, stream.channel.Id)
		s.lock.Unlock()

		stream.Close()
	}()

	return stream.w, nil
}

func newIngressStream(
	logger *zap.Logger,
	store *dao.ProfileStore,
	transfer control.TransferControl,
	network control.NetworkControl,
	directory control.DirectoryControl,
	addr *rtmpingress.StreamAddr,
	conn io.Closer,
) (s *ingressStream, err error) {
	ctx, cancel := context.WithCancel(context.Background())

	s = &ingressStream{
		logger:    logger,
		store:     store,
		transfer:  transfer,
		network:   network,
		directory: directory,

		ctx:       ctx,
		cancelCtx: cancel,

		startTime: timeutil.Now(),
		conn:      conn,
	}

	s.channel, err = dao.GetVideoChannelByStreamKey(store, addr.Key)
	if err != nil {
		return nil, fmt.Errorf("getting channel: %w", err)
	}

	mu := dao.NewMutex(logger, store, strconv.AppendUint(nil, s.channel.Id, 10))
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

	go func() {
		id, err := s.publishDirectoryListing()
		if err != nil {
			s.logger.Debug("publishing stream to directory failed", zap.Error(err))
		}
		// TODO: store somewhere for use with unpublish/snippet stream
		_ = id
	}()

	return s, nil
}

type ingressStream struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	transfer  control.TransferControl
	network   control.NetworkControl
	directory control.DirectoryControl

	ctx       context.Context
	cancelCtx context.CancelFunc
	closeOnce sync.Once

	startTime   timeutil.Time
	channel     *videov1.VideoChannel
	directoryID uint64
	conn        io.Closer

	swarm      *ppspp.Swarm
	transferID []byte
	w          ioutil.WriteFlusher
}

func (s *ingressStream) Close() {
	s.closeOnce.Do(func() {
		s.cancelCtx()
		s.conn.Close()

		s.transfer.Remove(s.transferID)
		s.unpublishDirectoryListing()
	})
}

func (s *ingressStream) UpdateChannel(channel *videov1.VideoChannel) {
	s.channel = channel
	s.publishDirectoryListing()
}

func (s *ingressStream) openWriter() (*ppspp.Swarm, ioutil.WriteFlusher, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:          1024,
			ChunksPerSignature: 32,
			StreamCount:        16,
			LiveWindow:         32 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
				MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
				LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
			},
		},
		Key: s.channel.Key,
	})
	if err != nil {
		return nil, nil, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.DefaultSize)
	if err != nil {
		return nil, nil, err
	}

	return w.Swarm(), cw, nil
}

func (s *ingressStream) channelNetworkKey() []byte {
	switch o := s.channel.Owner.(type) {
	case *videov1.VideoChannel_Local_:
		return o.Local.NetworkKey
	case *videov1.VideoChannel_LocalShare_:
		return dao.CertificateRoot(o.LocalShare.Certificate).Key
	default:
		panic("unsupported channel")
	}
}

func (s *ingressStream) channelCreatorCert() (*certificate.Certificate, error) {
	switch o := s.channel.Owner.(type) {
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
