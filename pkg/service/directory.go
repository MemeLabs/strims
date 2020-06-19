package service

import (
	"bytes"
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"github.com/sony/sonyflake"
	"google.golang.org/protobuf/proto"
)

var directorySalt = []byte("directory")

// Directory ...
type Directory interface {
	Publish(listing *pb.DirectoryListing) error
	Unpublish(key []byte) error
	Join(listingID uint64) error
	Part(listingID uint64) error
}

// NewDirectoryServer ...
func NewDirectoryServer(ctx context.Context, svc *NetworkServices, key *pb.Key) (*DirectoryServer, error) {
	ps, err := NewPubSubServer(ctx, svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	s := &DirectoryServer{
		ps:        ps,
		events:    make(chan *pb.DirectoryServerEvent),
		snowflake: sonyflake.NewSonyflake(sonyflake.Settings{}),
	}

	go s.transformDirectoryMessages(ctx, ps)

	return s, nil
}

// DirectoryServer ...
type DirectoryServer struct {
	closeOnce    sync.Once
	ps           *PubSubServer
	events       chan *pb.DirectoryServerEvent
	listingsLock sync.Mutex
	listings     directoryListingMap
	snowflake    *sonyflake.Sonyflake
}

// Close ...
func (s *DirectoryServer) Close() {
	s.closeOnce.Do(func() {
		s.ps.Close()
		close(s.events)
	})
}

// Events ...
func (s *DirectoryServer) Events() <-chan *pb.DirectoryServerEvent {
	return s.events
}

func (s *DirectoryServer) transformDirectoryMessages(ctx context.Context, ps *PubSubServer) {
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

	s.Close()
}

func verifyPublish(publish *pb.DirectoryClientEvent_Publish) bool {
	return true
}

func verifyUnpublish(publish *pb.DirectoryClientEvent_Unpublish) bool {
	return true
}

// Publish ...
func (s *DirectoryServer) Publish(listing *pb.DirectoryListing) error {
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
func (s *DirectoryServer) Unpublish(key []byte) error {
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
func (s *DirectoryServer) Join(listingID uint64) error {
	return nil
}

// Part ...
func (s *DirectoryServer) Part(listingID uint64) error {
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
func NewDirectoryClient(ctx context.Context, svc *NetworkServices, key []byte) (*DirectoryClient, error) {
	ps, err := NewPubSubClient(ctx, svc, key, directorySalt)
	if err != nil {
		return nil, err
	}

	c := &DirectoryClient{
		ps:     ps,
		events: make(chan *pb.DirectoryClientEvent),
	}

	go c.readDirectoryEvents(ctx, ps)

	return c, nil
}

// DirectoryClient ...
type DirectoryClient struct {
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

func (c *DirectoryClient) readDirectoryEvents(ctx context.Context, ps *PubSubClient) {
	for m := range ps.Messages() {
		e := &pb.DirectoryClientEvent{}
		if err := proto.Unmarshal(m.Body, e); err != nil {
			continue
		}
		c.events <- e
	}

	c.Close()
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
