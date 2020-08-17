package service

import (
	"bytes"
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
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
	Join(ctx context.Context, listingID uint64) error
	Part(ctx context.Context, listingID uint64) error
	Events() <-chan *pb.DirectoryServerEvent
}

// NewDirectoryServer ...
func NewDirectoryServer(logger *zap.Logger, lock *dao.Mutex, svc *NetworkServices, key *pb.Key) (*DirectoryServer, error) {
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
	go s.pumpEvents()

	return s, nil
}

// DirectoryServer ...
type DirectoryServer struct {
	lock          *dao.Mutex
	close         context.CancelFunc
	logger        *zap.Logger
	directoryLock sync.RWMutex
	directory     Directory
	events        chan *pb.DirectoryServerEvent
}

func (s *DirectoryServer) upgrade(ctx context.Context, svc *NetworkServices, key *pb.Key) {
	if err := s.lock.Lock(ctx); err != nil {
		s.Close()
		return
	}

	s.logger.Debug("upgrading directory server", logutil.ByteHex("networkKey", svc.Network.CAKey()))

	s.directoryLock.Lock()
	defer s.directoryLock.Unlock()

	s.directory.Close()

	server, err := newDirectoryServer(s.logger, svc, key)
	if err != nil {
		s.logger.Error("failed to start directory server", zap.Error(err))
		s.Close()
		return
	}

	s.directory = server
	go s.pumpEvents()
}

func (s *DirectoryServer) pumpEvents() {
	for event := range s.directory.Events() {
		s.events <- event
	}
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
func (s *DirectoryServer) Join(ctx context.Context, listingID uint64) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Join(ctx, listingID)
}

// Part ...
func (s *DirectoryServer) Part(ctx context.Context, listingID uint64) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Part(ctx, listingID)
}

// Events ...
func (s *DirectoryServer) Events() <-chan *pb.DirectoryServerEvent {
	return s.events
}

func newDirectoryServer(logger *zap.Logger, svc *NetworkServices, key *pb.Key) (*directoryServer, error) {
	ps, err := NewPubSubServer(svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	s := &directoryServer{
		logger: logger,
		ps:     ps,
		events: make(chan *pb.DirectoryServerEvent),
	}

	go s.transformDirectoryMessages(ps)

	return s, nil
}

type directoryServer struct {
	logger       *zap.Logger
	closeOnce    sync.Once
	ps           *PubSubServer
	events       chan *pb.DirectoryServerEvent
	listingsLock sync.Mutex
	listings     directoryListingMap
	nextID       uint64
}

// Close ...
func (s *directoryServer) Close() {
	s.closeOnce.Do(func() {
		s.ps.Close()
		close(s.events)
	})
}

// Events ...
func (s *directoryServer) Events() <-chan *pb.DirectoryServerEvent {
	return s.events
}

func (s *directoryServer) send(ctx context.Context, event *pb.DirectoryServerEvent) error {
	s.events <- event
	return sendProto(ctx, s.ps, event)
}

func (s *directoryServer) transformDirectoryMessages(ps *PubSubServer) {
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
				if err := s.Join(ctx, b.Join.ListingId); err != nil {
					s.logger.Debug("handling join failed", zap.Error(err))
				}
			case *pb.DirectoryClientEvent_Part_:
				if err := s.Part(ctx, b.Part.ListingId); err != nil {
					s.logger.Debug("handling part failed", zap.Error(err))
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

	s.listingsLock.Lock()
	defer s.listingsLock.Unlock()

	old, ok := s.listings.Get(listing.Key)
	if ok {
		listing.Id = old.Id
	} else {
		s.nextID++
		listing.Id = s.nextID
	}

	s.listings.Insert(listing.Key, listing)

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

	s.listingsLock.Lock()
	defer s.listingsLock.Unlock()

	listing, ok := s.listings.Get(key)
	if !ok {
		return nil
	}

	return s.send(ctx, &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Unpublish_{
			Unpublish: &pb.DirectoryServerEvent_Unpublish{
				ListingId: listing.Id,
			},
		},
	})
}

// Join ...
func (s *directoryServer) Join(ctx context.Context, listingID uint64) error {
	return nil
}

// Part ...
func (s *directoryServer) Part(ctx context.Context, listingID uint64) error {
	return nil
}

type directoryListingMap struct {
	m llrb.LLRB
}

func (m *directoryListingMap) Insert(k []byte, v *pb.DirectoryListing) {
	m.m.InsertNoReplace(directoryListingMapItem{k, v})
}

func (m *directoryListingMap) Delete(k []byte) {
	m.m.Delete(directoryListingMapItem{k, nil})
}

func (m *directoryListingMap) Get(k []byte) (*pb.DirectoryListing, bool) {
	if it := m.m.Get(directoryListingMapItem{k, nil}); it != nil {
		return it.(directoryListingMapItem).v, true
	}
	return nil, false
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
func NewDirectoryClient(logger *zap.Logger, svc *NetworkServices, key []byte) (*DirectoryClient, error) {
	ps, err := NewPubSubClient(svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	logger.Debug("starting directory client", logutil.ByteHex("network", svc.Network.CAKey()))

	c := &DirectoryClient{
		logger: logger,
		ps:     ps,
		events: make(chan *pb.DirectoryServerEvent),
	}

	go c.readDirectoryEvents(ps)

	return c, nil
}

// DirectoryClient ...
type DirectoryClient struct {
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *PubSubClient
	events    chan *pb.DirectoryServerEvent
}

// Close ...
func (c *DirectoryClient) Close() {
	c.closeOnce.Do(func() {
		c.ps.Close()
		close(c.events)
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
func (c *DirectoryClient) Join(ctx context.Context, listingID uint64) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Join_{
			Join: &pb.DirectoryClientEvent_Join{
				ListingId: listingID,
			},
		},
	})
}

// Part ...
func (c *DirectoryClient) Part(ctx context.Context, listingID uint64) error {
	return sendProto(ctx, c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Part_{
			Part: &pb.DirectoryClientEvent_Part{
				ListingId: listingID,
			},
		},
	})
}

// Events ...
func (c *DirectoryClient) Events() <-chan *pb.DirectoryServerEvent {
	return c.events
}

func (c *DirectoryClient) readDirectoryEvents(ps *PubSubClient) {
	for m := range ps.Messages() {
		e := &pb.DirectoryServerEvent{}
		if err := proto.Unmarshal(m.Body, e); err != nil {
			continue
		}
		c.events <- e
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
