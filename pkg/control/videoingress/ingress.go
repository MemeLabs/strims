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

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"go.uber.org/zap"
)

const streamLockTimeout = time.Minute * 2
const streamUpdateInterval = time.Minute

func newIngressService(
	logger *zap.Logger,
	store *dao.ProfileStore,
	transfer *transfer.Control,
	network *network.Control,
	directory *directory.Control,
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
	transfer   *transfer.Control
	network    *network.Control
	directory  *directory.Control
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
		s.transcoder,
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
		if err := s.transcoder.Transcode(a.URI, a.Key, "source", stream.w); err != nil {
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

func newIngressStream(
	logger *zap.Logger,
	store *dao.ProfileStore,
	transfer *transfer.Control,
	network *network.Control,
	directory *directory.Control,
	transcoder *rtmpingress.Transcoder,
	addr *rtmpingress.StreamAddr,
	conn io.Closer,
) (s *ingressStream, err error) {
	ctx, cancel := context.WithCancel(context.Background())

	s = &ingressStream{
		logger:     logger,
		store:      store,
		transfer:   transfer,
		network:    network,
		directory:  directory,
		transcoder: transcoder,

		ctx:       ctx,
		cancelCtx: cancel,

		startTime: time.Now(),
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
		// TODO: retry/refresh periodically
		if err := s.publishDirectoryListing(); err != nil {
			s.logger.Debug("publishing stream to directory failed", zap.Error(err))
		}
	}()

	return s, nil
}

type ingressStream struct {
	logger     *zap.Logger
	store      *dao.ProfileStore
	transfer   *transfer.Control
	network    *network.Control
	directory  *directory.Control
	transcoder *rtmpingress.Transcoder

	ctx       context.Context
	cancelCtx context.CancelFunc
	closeOnce sync.Once

	startTime time.Time
	channel   *videov1.VideoChannel
	conn      io.Closer

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
			ChunksPerSignature: 64,
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

	cw, err := chunkstream.NewWriterSize(w, chunkstream.MaxSize)
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
		return dao.GetRootCert(o.LocalShare.Certificate).Key
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

func (s *ingressStream) publishDirectoryListing() error {
	creator, err := s.channelCreatorCert()
	if err != nil {
		return err
	}

	listing := &networkv1.DirectoryListing{
		Creator:   creator,
		Timestamp: time.Now().Unix(),
		Snippet:   s.channel.DirectoryListingSnippet,
		Content: &networkv1.DirectoryListing_Media{
			Media: &networkv1.DirectoryListingMedia{
				StartedAt: s.startTime.Unix(),
				MimeType:  rtmpingress.TranscoderMimeType,
				SwarmUri:  s.swarm.URI().String(),
			},
		},
	}
	if err := dao.SignMessage(listing, s.channel.Key); err != nil {
		return err
	}

	return s.directory.Publish(context.Background(), listing, s.channelNetworkKey())
}

func (s *ingressStream) unpublishDirectoryListing() error {
	return s.directory.Unpublish(context.Background(), s.channel.Key.Public, s.channelNetworkKey())
}
