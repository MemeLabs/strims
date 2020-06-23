package service

import (
	"bytes"
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"github.com/sony/sonyflake"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var directorySalt = []byte("directory")

// Directory ...
type Directory interface {
	Close()
	Publish(listing *pb.DirectoryListing) error
	Unpublish(key []byte) error
	Join(listingID uint64) error
	Part(listingID uint64) error
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
	}

	go s.upgrade(ctx, svc, key)

	return s, nil
}

// DirectoryServer ...
type DirectoryServer struct {
	lock          *dao.Mutex
	close         context.CancelFunc
	logger        *zap.Logger
	directoryLock sync.RWMutex
	directory     Directory
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
}

// Close ...
func (s *DirectoryServer) Close() {
	s.close()
	s.lock.Release()

	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	s.directory.Close()
}

// Publish ...
func (s *DirectoryServer) Publish(listing *pb.DirectoryListing) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Publish(listing)
}

// Unpublish ...
func (s *DirectoryServer) Unpublish(key []byte) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Unpublish(key)
}

// Join ...
func (s *DirectoryServer) Join(listingID uint64) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Join(listingID)
}

// Part ...
func (s *DirectoryServer) Part(listingID uint64) error {
	s.directoryLock.RLock()
	defer s.directoryLock.RUnlock()
	return s.directory.Part(listingID)
}

func newDirectoryServer(logger *zap.Logger, svc *NetworkServices, key *pb.Key) (*directoryServer, error) {
	ps, err := NewPubSubServer(svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	s := &directoryServer{
		logger:    logger,
		ps:        ps,
		events:    make(chan *pb.DirectoryClientEvent),
		snowflake: sonyflake.NewSonyflake(sonyflake.Settings{}),
	}

	go s.transformDirectoryMessages(ps)

	return s, nil
}

type directoryServer struct {
	logger       *zap.Logger
	closeOnce    sync.Once
	ps           *PubSubServer
	events       chan *pb.DirectoryClientEvent
	listingsLock sync.Mutex
	listings     directoryListingMap
	snowflake    *sonyflake.Sonyflake
}

// Close ...
func (s *directoryServer) Close() {
	s.closeOnce.Do(func() {
		s.ps.Close()
		close(s.events)
	})
}

// Events ...
func (s *directoryServer) Events() <-chan *pb.DirectoryClientEvent {
	return s.events
}

func (s *directoryServer) transformDirectoryMessages(ps *PubSubServer) {
	for p := range ps.Messages() {
		var e pb.DirectoryClientEvent
		if err := proto.Unmarshal(p.Body, &e); err != nil {
			continue
		}

		switch b := e.Body.(type) {
		case *pb.DirectoryClientEvent_Publish_:
			if verifyPublish(b.Publish) {
				s.Publish(b.Publish.Listing)
			}
		case *pb.DirectoryClientEvent_Unpublish_:
			if verifyUnpublish(b.Unpublish) {
				s.Unpublish(b.Unpublish.Key)
			}
		case *pb.DirectoryClientEvent_Join_:
			s.Join(b.Join.ListingId)
		case *pb.DirectoryClientEvent_Part_:
			s.Part(b.Part.ListingId)
		}
	}
}

func verifyPublish(publish *pb.DirectoryClientEvent_Publish) bool {
	return true
}

func verifyUnpublish(publish *pb.DirectoryClientEvent_Unpublish) bool {
	return true
}

// Publish ...
func (s *directoryServer) Publish(listing *pb.DirectoryListing) error {
	// TODO: verify signature...

	s.listingsLock.Lock()
	defer s.listingsLock.Unlock()

	old, ok := s.listings.Get(listing.Key)
	if ok {
		listing.Id = old.Id
	} else {
		id, err := s.snowflake.NextID()
		if err != nil {
			return err
		}
		listing.Id = id
	}

	s.listings.Insert(listing.Key, listing)

	return sendProto(s.ps, &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Publish_{
			Publish: &pb.DirectoryServerEvent_Publish{
				Listing: listing,
			},
		},
	})
}

// Unpublish ...
func (s *directoryServer) Unpublish(key []byte) error {
	// TODO: signature

	s.listingsLock.Lock()
	defer s.listingsLock.Unlock()

	listing, ok := s.listings.Get(key)
	if !ok {
		return nil
	}

	return sendProto(s.ps, &pb.DirectoryServerEvent{
		Body: &pb.DirectoryServerEvent_Unpublish_{
			Unpublish: &pb.DirectoryServerEvent_Unpublish{
				ListingId: listing.Id,
			},
		},
	})
}

// Join ...
func (s *directoryServer) Join(listingID uint64) error {
	return nil
}

// Part ...
func (s *directoryServer) Part(listingID uint64) error {
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
		events: make(chan *pb.DirectoryClientEvent),
	}

	go c.readDirectoryEvents(ps)

	return c, nil
}

// DirectoryClient ...
type DirectoryClient struct {
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *PubSubClient
	events    chan *pb.DirectoryClientEvent
}

// Close ...
func (c *DirectoryClient) Close() {
	c.closeOnce.Do(func() {
		c.ps.Close()
		close(c.events)
	})
}

// Publish ...
func (c *DirectoryClient) Publish(listing *pb.DirectoryListing) error {
	return sendProto(c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Publish_{
			Publish: &pb.DirectoryClientEvent_Publish{
				Listing: listing,
			},
		},
	})
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(key []byte) error {
	return sendProto(c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Unpublish_{
			Unpublish: &pb.DirectoryClientEvent_Unpublish{
				Key: key,
			},
		},
	})
}

// Join ...
func (c *DirectoryClient) Join(listingID uint64) error {
	return sendProto(c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Join_{
			Join: &pb.DirectoryClientEvent_Join{
				ListingId: listingID,
			},
		},
	})
}

// Part ...
func (c *DirectoryClient) Part(listingID uint64) error {
	return sendProto(c.ps, &pb.DirectoryClientEvent{
		Body: &pb.DirectoryClientEvent_Part_{
			Part: &pb.DirectoryClientEvent_Part{
				ListingId: listingID,
			},
		},
	})
}

// Events ...
func (c *DirectoryClient) Events() <-chan *pb.DirectoryClientEvent {
	return c.events
}

func (c *DirectoryClient) readDirectoryEvents(ps *PubSubClient) {
	for m := range ps.Messages() {
		e := &pb.DirectoryClientEvent{}
		if err := proto.Unmarshal(m.Body, e); err != nil {
			continue
		}
		c.events <- e
	}
}

type pubSub interface {
	Send(key string, b []byte) error
}

func sendProto(ps pubSub, m proto.Message) error {
	b, err := proto.Marshal(m)
	if err != nil {
		return err
	}

	return ps.Send("", b)
}
