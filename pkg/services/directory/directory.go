package network

import (
	"bytes"
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/service/pubsub"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const directoryPingInterval = 5 * time.Minute

var directorySalt = []byte("directory")

// Directory ...
type Directory interface {
	Close()
	Publish(ctx context.Context, listing *pb.DirectoryListing) error
	Unpublish(ctx context.Context, key []byte) error
	Join(ctx context.Context, key []byte) error
	Part(ctx context.Context, key []byte) error
	NotifyEvents(chan *pb.DirectoryServerEvent)
	StopNotifyingEvents(chan *pb.DirectoryServerEvent)
}

// NewDirectoryServer ...
func NewDirectoryServer(logger *zap.Logger, lock *dao.Mutex, svc *Services, key *pb.Key) (*DirectoryServer, error) {
	client, err := NewDirectoryClient(logger, svc, key.Public)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	s := &DirectoryServer{
		lock:      lock,
		close:     cancel,
		logger:    logger,
		directory: client,
		events:    make(chan *pb.DirectoryServerEvent),
	}

	go s.upgrade(ctx, svc, key)

	return s, nil
}

// Server ...
type DirectoryServer struct {
	lock          *dao.Mutex
	close         context.CancelFunc
	logger        *zap.Logger
	directoryLock sync.RWMutex
	directory     Directory
	events        chan *pb.DirectoryServerEvent
}

func (s *DirectoryServer) upgrade(ctx context.Context, svc *Services, key *pb.Key) {
	if err := s.lock.Lock(ctx); err != nil {
		s.Close()
		return
	}

	s.logger.Debug("upgrading directory server", logutil.ByteHex("networkKey", svc.Network.Key()))

	s.directoryLock.Lock()
	defer s.directoryLock.Unlock()

	client := s.directory.(*DirectoryClient)
	client.ps.Close()

	ps, err := pubsub.NewServer(svc, key, directorySalt)
	if err != nil {
		s.logger.Error("failed to start directory server", zap.Error(err))
		s.Close()
		return
	}

	server := &directoryServer{
		directoryListingMap: client.directoryListingMap,
		logger:              s.logger,
		ps:                  ps,
	}
	go server.transformDirectoryMessages(ps)

	s.directory = server
}

// Close ...
func (s *DirectoryServer) Close() {
	s.close()
	if err := s.lock.Release(); err != nil {
		s.logger.Error("failed to release lock", zap.Error(err))
	}

	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	s.directory.Close()
}

// Publish ...
func (s *DirectoryServer) Publish(ctx context.Context, listing *pb.DirectoryListing) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Publish(ctx, listing)
}

// Unpublish ...
func (s *DirectoryServer) Unpublish(ctx context.Context, key []byte) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Unpublish(ctx, key)
}

// Join ...
func (s *DirectoryServer) Join(ctx context.Context, key []byte) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Join(ctx, key)
}

// Part ...
func (s *DirectoryServer) Part(ctx context.Context, key []byte) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Part(ctx, key)
}

// NotifyEvents ...
func (s *DirectoryServer) NotifyEvents(ch chan *pb.DirectoryServerEvent) {
	s.directory.NotifyEvents(ch)
}

// StopNotifyingEvents ...
func (s *DirectoryServer) StopNotifyingEvents(ch chan *pb.DirectoryServerEvent) {
	s.directory.StopNotifyingEvents(ch)
}

type directoryServer struct {
	*directoryListingMap
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *pubsub.Server
}

// Close ...
func (s *directoryServer) Close() {
	s.closeOnce.Do(func() {
		s.ps.Close()
	})
}

func (s *directoryServer) send(ctx context.Context, event *pb.DirectoryServerEvent) error {
	s.events.Emit(event)
	return sendProto(ctx, s.ps, event)
}

func (s *directoryServer) transformDirectoryMessages(ps *pubsub.Server) {
	ticker := time.NewTicker(directoryPingInterval)
	for {
		select {
		case t := <-ticker.C:
			if err := s.ping(t); err != nil {
				s.logger.Error("failed to send ping", zap.Error(err))
			}
		case p := <-ps.Messages():
			var e pb.DirectoryClientEvent
			if err := proto.Unmarshal(p.Body, &e); err != nil {
				continue
			}

			ctx := context.Background()
			switch b := e.Body.(type) {
			case *pb.DirectoryClientEvent_Publish_:
				if verifyPublish(b.Publish) {
					if err := s.Publish(ctx, b.Publish.Listing); err != nil {
						s.logger.Debug("handling publish failed", zap.Error(err))
					}
				}
			case *pb.DirectoryClientEvent_Unpublish_:
				if verifyUnpublish(b.Unpublish) {
					if err := s.Unpublish(ctx, b.Unpublish.Key); err != nil {
						s.logger.Debug("handling unpublish failed", zap.Error(err))
					}
				}
			case *pb.DirectoryClientEvent_Join_:
				if err := s.Join(ctx, b.Join.Key); err != nil {
					s.logger.Debug("handling join failed", zap.Error(err))
				}
			case *pb.DirectoryClientEvent_Part_:
				if err := s.Part(ctx, b.Part.Key); err != nil {
					s.logger.Debug("handling part failed", zap.Error(err))
				}
			case *pb.DirectoryClientEvent_Ping_:
				if err := s.Ping(ctx); err != nil {
					s.logger.Debug("handling ping failed", zap.Error(err))
				}
			}
		}
	}
}

func verifyPublish(publish *pb.DirectoryClientEvent_Publish) bool {
	return true
}

func verifyUnpublish(publish *pb.DirectoryClientEvent_Unpublish) bool {
	return true
}

func (s *directoryServer) ping(t time.Time) error {
	return s.send(context.Background(), &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Ping_{
			Ping: &pb.DirectoryServerEvent_Ping{
				Time: t.Unix(),
			},
		},
	})
}

// Publish ...
func (s *directoryServer) Publish(ctx context.Context, listing *pb.DirectoryListing) error {
	// TODO: verify signature...

	s.logger.Debug("received publish", logutil.ByteHex("key", listing.Key))

	s.Insert(listing.Key, listing)

	return s.send(ctx, &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Publish_{
			Publish: &pb.DirectoryServerEvent_Publish{
				Listing: listing,
			},
		},
	})
}

// Unpublish ...
func (s *directoryServer) Unpublish(ctx context.Context, key []byte) error {
	// TODO: signature

	s.logger.Debug("received unpublish", logutil.ByteHex("key", key))

	listing, ok := s.Get(key)
	if !ok {
		return nil
	}
	s.Delete(key)

	return s.send(ctx, &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Unpublish_{
			Unpublish: &pb.DirectoryServerEvent_Unpublish{
				Key: listing.Key,
			},
		},
	})
}

// Join ...
func (s *directoryServer) Join(ctx context.Context, key []byte) error {
	return nil
}

// Part ...
func (s *directoryServer) Part(ctx context.Context, key []byte) error {
	return nil
}

// Ping ...
func (s *directoryServer) Ping(ctx context.Context) error {
	return nil
}

type directoryListingMap struct {
	lock   sync.Mutex
	m      llrb.LLRB
	events event.Observer
}

func (m *directoryListingMap) Insert(k []byte, v *pb.DirectoryListing) {
	m.lock.Lock()
	defer m.lock.Unlock()
	m.m.ReplaceOrInsert(directoryListingMapItem{k, v})
}

func (m *directoryListingMap) Delete(k []byte) {
	m.lock.Lock()
	defer m.lock.Unlock()
	m.m.Delete(directoryListingMapItem{k, nil})
}

func (m *directoryListingMap) Get(k []byte) (*pb.DirectoryListing, bool) {
	m.lock.Lock()
	defer m.lock.Unlock()

	if it := m.m.Get(directoryListingMapItem{k, nil}); it != nil {
		return it.(directoryListingMapItem).v, true
	}
	return nil, false
}

// NotifyEvents ...
func (m *directoryListingMap) NotifyEvents(ch chan *pb.DirectoryServerEvent) {
	m.lock.Lock()
	defer m.lock.Unlock()

	m.m.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		ch <- &pb.DirectoryServerEvent{
			Body: &pb.DirectoryServerEvent_Publish_{
				Publish: &pb.DirectoryServerEvent_Publish{
					Listing: i.(directoryListingMapItem).v,
				},
			},
		}
		return true
	})

	m.events.Notify(ch)
}

// StopNotifyingEvents ...
func (m *directoryListingMap) StopNotifyingEvents(ch chan *pb.DirectoryServerEvent) {
	m.events.StopNotifying(ch)
}

type directoryListingMapItem struct {
	k []byte
	v *pb.DirectoryListing
}

func (t directoryListingMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(directoryListingMapItem); ok {
		return bytes.Compare(t.k, o.k) == -1
	}
	return !oi.Less(t)
}

// NewDirectoryClient ...
func NewDirectoryClient(logger *zap.Logger, svc *Services, key []byte) (*DirectoryClient, error) {
	ps, err := pubsub.NewClient(svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	logger.Debug("starting directory client", logutil.ByteHex("network", svc.Network.Key()))

	c := &DirectoryClient{
		directoryListingMap: new(directoryListingMap),
		logger:              logger,
		ps:                  ps,
	}

	go c.readDirectoryEvents(ps)

	return c, nil
}

// DirectoryClient ...
type DirectoryClient struct {
	*directoryListingMap
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *pubsub.Client
}

// Close ...
func (c *DirectoryClient) Close() {
	c.closeOnce.Do(func() {
		c.ps.Close()
		c.events.Close()
	})
}

// Publish ...
func (c *DirectoryClient) Publish(ctx context.Context, listing *pb.DirectoryListing) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Publish_{
			Publish: &pb.DirectoryClientEvent_Publish{
				Listing: listing,
			},
		},
	})
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(ctx context.Context, key []byte) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Unpublish_{
			Unpublish: &pb.DirectoryClientEvent_Unpublish{
				Key: key,
			},
		},
	})
}

// Join ...
func (c *DirectoryClient) Join(ctx context.Context, key []byte) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Join_{
			Join: &pb.DirectoryClientEvent_Join{
				Key: key,
			},
		},
	})
}

// Part ...
func (c *DirectoryClient) Part(ctx context.Context, key []byte) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Part_{
			Part: &pb.DirectoryClientEvent_Part{
				Key: key,
			},
		},
	})
}

func (c *DirectoryClient) readDirectoryEvents(ps *pubsub.Client) {

	for m := range ps.Messages() {
		e := &pb.DirectoryServerEvent{}
		if err := proto.Unmarshal(m.Body, e); err != nil {
			c.logger.Debug("failed to decode directory event", zap.Error(err))
			continue
		}

		c.handleDirectoryEvent(e)
		c.events.Emit(e)
	}
}

func (c *DirectoryClient) handleDirectoryEvent(e *pb.DirectoryServerEvent) {
	switch b := e.Body.(type) {
	case *pb.DirectoryServerEvent_Publish_:
		c.logger.Debug("received publish", logutil.ByteHex("key", b.Publish.Listing.Key))
		c.Insert(b.Publish.Listing.Key, b.Publish.Listing)
	case *pb.DirectoryServerEvent_Unpublish_:
		c.logger.Debug("received unpublish", logutil.ByteHex("key", b.Unpublish.Key))
		c.Delete(b.Unpublish.Key)
	}
}

type pubSub interface {
	Send(ctx context.Context, key string, b []byte) error
}

func sendProto(ctx context.Context, ps pubSub, m proto.Message) error {
	b := pool.Get(uint16(proto.Size(m)))
	defer pool.Put(b)

	_, err := proto.MarshalOptions{}.MarshalAppend((*b)[:0], m)
	if err != nil {
		return err
	}

	return ps.Send(ctx, "", *b)
}
