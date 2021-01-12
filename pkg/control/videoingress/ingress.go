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
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
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

func newIngressService(logger *zap.Logger, store *dao.ProfileStore, transfer *transfer.Control, dialer *dialer.Control, network *network.Control) *ingressService {
	return &ingressService{
		logger:     logger,
		store:      store,
		transfer:   transfer,
		dialer:     dialer,
		network:    network,
		transcoder: rtmpingress.NewTranscoder(logger),
		streams:    map[uint64]*ingressStream{},
	}
}

type ingressService struct {
	logger     *zap.Logger
	store      *dao.ProfileStore
	transfer   *transfer.Control
	dialer     *dialer.Control
	network    *network.Control
	transcoder *rtmpingress.Transcoder

	lock    sync.Mutex
	streams map[uint64]*ingressStream
}

func (s *ingressService) UpdateChannel(channel *videov1.VideoChannel) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if stream, ok := s.streams[channel.Id]; ok {
		stream.publishDirectoryListing(channel)
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

	stream, err := newIngressStream(s.logger, s.store, s.transfer, s.dialer, s.network, s.transcoder, a, c)
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
	dialer *dialer.Control,
	network *network.Control,
	transcoder *rtmpingress.Transcoder,
	addr *rtmpingress.StreamAddr,
	conn io.Closer,
) (s *ingressStream, err error) {
	ctx, cancel := context.WithCancel(context.Background())

	s = &ingressStream{
		logger:     logger,
		store:      store,
		transfer:   transfer,
		dialer:     dialer,
		network:    network,
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

	s.swarm, s.w, err = s.openWriter(s.channel.Key)
	if err != nil {
		s.Close()
		return nil, fmt.Errorf("opening output stream: %w", err)
	}

	s.transferID = s.transfer.Add(s.swarm, []byte{})
	s.transfer.Publish(s.transferID, s.channelNetworkKey(s.channel))

	if err := s.publishDirectoryListing(s.channel); err != nil {
		s.Close()
		return nil, fmt.Errorf("publishing stream to directory: %w", err)
	}

	return s, nil
}

type ingressStream struct {
	logger     *zap.Logger
	store      *dao.ProfileStore
	transfer   *transfer.Control
	dialer     *dialer.Control
	network    *network.Control
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

func (s *ingressStream) openWriter(key *key.Key) (*ppspp.Swarm, ioutil.WriteFlusher, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:          1024,
			ChunksPerSignature: 32,
			LiveWindow:         32 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
				MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
				LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
			},
		},
		Key: key,
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

func (s *ingressStream) channelNetworkKey(channel *videov1.VideoChannel) []byte {
	switch o := channel.Owner.(type) {
	case *videov1.VideoChannel_Local_:
		return o.Local.NetworkKey
	case *videov1.VideoChannel_LocalShare_:
		return dao.GetRootCert(o.LocalShare.Certificate).Key
	default:
		panic("unsupported channel")
	}
}

func (s *ingressStream) channelCreatorCert(channel *videov1.VideoChannel) (*certificate.Certificate, error) {
	switch o := channel.Owner.(type) {
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

func (s *ingressStream) publishDirectoryListing(channel *videov1.VideoChannel) error {
	networkKey := s.channelNetworkKey(channel)
	creator, err := s.channelCreatorCert(channel)
	if err != nil {
		return err
	}

	client, err := s.dialer.Client(networkKey, networkKey, directory.AddressSalt)
	if err != nil {
		return err
	}

	listing := &networkv1.DirectoryListing{
		Creator:   creator,
		Timestamp: time.Now().Unix(),
		Snippet:   channel.DirectoryListingSnippet,
		Content: &networkv1.DirectoryListing_Media{
			Media: &networkv1.DirectoryListingMedia{
				StartedAt: s.startTime.Unix(),
				MimeType:  rtmpingress.TranscoderMimeType,
				SwarmUri:  s.swarm.URI().String(),
			},
		},
	}
	if err := dao.SignMessage(listing, channel.Key); err != nil {
		return err
	}

	// TODO: move this to directory controller using reference counted clients to ping
	return networkv1.NewDirectoryClient(client).Publish(
		context.Background(),
		&networkv1.DirectoryPublishRequest{Listing: listing},
		&networkv1.DirectoryPublishResponse{},
	)
}

func (s *ingressStream) unpublishDirectoryListing() error {
	networkKey := s.channelNetworkKey(s.channel)
	client, err := s.dialer.Client(networkKey, networkKey, directory.AddressSalt)
	if err != nil {
		return err
	}

	return networkv1.NewDirectoryClient(client).Unpublish(
		context.Background(),
		&networkv1.DirectoryUnpublishRequest{Key: s.channel.Key.Public},
		&networkv1.DirectoryUnpublishResponse{},
	)
}
