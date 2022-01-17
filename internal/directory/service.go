package directory

import (
	"context"
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/set"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/crypto/blake2b"
	"google.golang.org/protobuf/proto"
)

const (
	broadcastInterval = time.Second
	sessionTimeout    = time.Minute * 15
	pingStartupDelay  = time.Second * 30
	minPingInterval   = time.Minute * 10
	maxPingInterval   = time.Minute * 14
	embedLoadInterval = time.Second * 15
	refreshInterval   = time.Minute
)

// AddressSalt ...
var AddressSalt = []byte("directory")

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

const chunkSize = 1024

var swarmOptions = ppspp.SwarmOptions{
	ChunkSize:  chunkSize,
	LiveWindow: 2 * 1024,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

func newDirectoryService(logger *zap.Logger, dialer network.Dialer, key *key.Key, config *networkv1directory.ServerConfig, ew *protoutil.ChunkStreamWriter) *directoryService {
	return &directoryService{
		logger:          logger,
		dialer:          dialer,
		key:             key,
		config:          config,
		done:            make(chan struct{}),
		broadcastTicker: timeutil.DefaultTickEmitter.Ticker(broadcastInterval),
		embedLoadTicker: timeutil.DefaultTickEmitter.Ticker(embedLoadInterval),
		eventWriter:     ew,
		embedLoader:     newEmbedLoader(logger, config.Integrations),
		listings:        newIndexedLRU(),
	}
}

type directoryService struct {
	logger            *zap.Logger
	dialer            network.Dialer
	key               *key.Key
	config            *networkv1directory.ServerConfig
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   timeutil.Ticker
	embedLoadTicker   timeutil.Ticker
	lastBroadcastTime timeutil.Time
	lastRefreshTime   timeutil.Time
	eventWriter       *protoutil.ChunkStreamWriter
	embedLoader       *embedLoader
	lock              sync.Mutex
	nextID            uint64
	listings          indexedLRU
	sessions          lru
	users             lru
	certificate       *certificate.Certificate
}

func (d *directoryService) Run(ctx context.Context) error {
	defer d.Close()

	for {
		select {
		case now := <-d.broadcastTicker.C:
			if err := d.broadcast(now); err != nil {
				d.logger.Debug("directory broadcast failed", zap.Error(err))
			}
		case now := <-d.embedLoadTicker.C:
			if err := d.loadEmbeds(ctx, now); err != nil {
				d.logger.Debug("directory embed loading failed", zap.Error(err))
			}
		case <-d.done:
			return errors.New("closed")
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func (d *directoryService) Close() {
	d.closeOnce.Do(func() {
		d.broadcastTicker.Stop()
		d.embedLoadTicker.Stop()
		close(d.done)
	})
}

func (d *directoryService) broadcast(now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	var events []*networkv1directory.Event

	if d.lastRefreshTime.Add(refreshInterval).Before(now) {
		for it := d.listings.IterateTouchedBefore(d.lastBroadcastTime); it.Next(); {
			l := it.Value().(*listing)
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_ListingChange_{
					ListingChange: &networkv1directory.Event_ListingChange{
						Id:      l.id,
						Listing: l.listing,
						Snippet: l.snippet,
					},
				},
			})
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_ViewerCountChange_{
					ViewerCountChange: &networkv1directory.Event_ViewerCountChange{
						Id:    l.id,
						Count: uint32(l.publishers.Len() + l.viewers.Len()),
					},
				},
			})
		}

		for it := d.users.IterateTouchedBefore(d.lastBroadcastTime); it.Next(); {
			events = d.appendUserEvent(events, it.Value().(*user))
		}

		d.lastRefreshTime = now
	}

	for it := d.listings.IterateTouchedAfter(d.lastBroadcastTime); it.Next(); {
		l := it.Value().(*listing)

		if l.publishers.Len() == 0 && l.viewers.Len() == 0 {
			d.listings.Delete(l)
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_Unpublish_{
					Unpublish: &networkv1directory.Event_Unpublish{
						Id: l.id,
					},
				},
			})
		} else if !l.modifiedTime.Before(d.lastBroadcastTime) {
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_ListingChange_{
					ListingChange: &networkv1directory.Event_ListingChange{
						Id:      l.id,
						Listing: l.listing,
						Snippet: l.snippet,
					},
				},
			})
		} else {
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_ViewerCountChange_{
					ViewerCountChange: &networkv1directory.Event_ViewerCountChange{
						Id:    l.id,
						Count: uint32(l.publishers.Len() + l.viewers.Len()),
					},
				},
			})
		}
	}

	for it := d.users.IterateTouchedAfter(d.lastBroadcastTime); it.Next(); {
		events = d.appendUserEvent(events, it.Value().(*user))
	}

	eol := now.Add(-sessionTimeout)

	for s, ok := d.sessions.Pop(eol).(*session); ok; s, ok = d.sessions.Pop(eol).(*session) {
		u, userDeleted := d.deleteSession(s)
		if !userDeleted {
			continue
		}

		events = append(events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ViewerStateChange_{
				ViewerStateChange: &networkv1directory.Event_ViewerStateChange{
					Subject: u.certificate.Subject,
					Online:  false,
				},
			},
		})
	}

	for l, ok := d.listings.Pop(eol).(*listing); ok; l, ok = d.listings.Pop(eol).(*listing) {
		events = append(events, &networkv1directory.Event{
			Body: &networkv1directory.Event_Unpublish_{
				Unpublish: &networkv1directory.Event_Unpublish{
					Id: l.id,
				},
			},
		})
	}

	if events != nil {
		err := d.eventWriter.Write(&networkv1directory.EventBroadcast{
			Events: events,
		})
		if err != nil {
			return err
		}
	}

	d.lastBroadcastTime = now
	return nil
}

func (d *directoryService) appendUserEvent(events []*networkv1directory.Event, u *user) []*networkv1directory.Event {
	ids := set.New[uint64](8)
	u.sessions.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		it.(*session).viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			ids.Insert(it.(*listing).id)
			return true
		})
		it.(*session).publishedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			ids.Insert(it.(*listing).id)
			return true
		})
		return true
	})

	return append(events, &networkv1directory.Event{
		Body: &networkv1directory.Event_ViewerStateChange_{
			ViewerStateChange: &networkv1directory.Event_ViewerStateChange{
				Subject:    u.certificate.Subject,
				Online:     true,
				ViewingIds: ids.Slice(),
			},
		},
	})
}

func (d *directoryService) deleteSession(s *session) (*user, bool) {
	u := d.users.GetOrInsert(&user{certificate: s.certificate.GetParent()}).(*user)
	u.sessions.Delete(s)

	s.viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		it.(*listing).viewers.Delete(s)
		d.listings.Touch(it.(*listing))
		return true
	})
	s.publishedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		it.(*listing).publishers.Delete(s)
		d.listings.Touch(it.(*listing))
		return true
	})

	if u.sessions.Len() != 0 {
		return u, false
	}
	d.users.Delete(u)
	return u, true
}

func (d *directoryService) loadEmbeds(ctx context.Context, now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	embedIDs := map[networkv1directory.Listing_Embed_Service][]string{}

	d.listings.Each(func(i keyer) bool {
		if embed := i.(*listing).listing.GetEmbed(); embed != nil {
			embedIDs[embed.Service] = append(embedIDs[embed.Service], embed.Id)
		}
		return true
	})

	go func() {
		res := d.embedLoader.Load(ctx, embedIDs)
		now := timeutil.Now()

		d.lock.Lock()
		defer d.lock.Unlock()

		d.listings.Each(func(i keyer) bool {
			l := i.(*listing)
			if embed := l.listing.GetEmbed(); embed != nil {
				if svcSnippets, ok := res[embed.Service]; ok {
					if snippet, ok := svcSnippets[embed.Id]; ok {
						l.snippet = snippet
						l.modifiedTime = now
						d.listings.Touch(i)
					}
				}
			}
			return true
		})
	}()

	return nil
}

func (d *directoryService) mergeSnippet(listingID uint64, delta *networkv1directory.ListingSnippetDelta) bool {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(listingID).(*listing)
	if !ok {
		return false
	}

	mergeSnippet(l.nextSnippet, delta)
	if err := dao.VerifyMessage(l.nextSnippet); err != nil {
		return false
	}

	mergeSnippet(l.snippet, diffSnippets(l.snippet, l.nextSnippet))
	l.modifiedTime = timeutil.Now()
	d.listings.Touch(l)
	return true
}

func (d *directoryService) loadMediaEmbed(ctx context.Context, listingID uint64, swarmID ppspp.SwarmID, candidate kademlia.ID) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	client, err := d.dialer.ClientWithHostAddr(ctx, d.key.Public, candidate, vnic.SnippetPort)
	if err != nil {
		return nil
	}
	defer client.Close()

	snippetClient := networkv1directory.NewDirectorySnippetClient(client)

	ch := make(chan *networkv1directory.SnippetSubscribeResponse, 16)
	go func() {
		for {
			select {
			case res, ok := <-ch:
				if !ok || res.SnippetDelta == nil {
					cancel()
					return
				}
				if d.mergeSnippet(listingID, res.SnippetDelta) {
					// TODO: move on to another candidate if we haven't received a
					// successful update after... some timeout
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	req := &networkv1directory.SnippetSubscribeRequest{
		SwarmId: swarmID.Binary(),
	}
	return snippetClient.Subscribe(ctx, req, ch)
}

func (d *directoryService) Publish(ctx context.Context, req *networkv1directory.PublishRequest) (*networkv1directory.PublishResponse, error) {
	switch c := req.Listing.Content.(type) {
	case *networkv1directory.Listing_Embed_:
		if !d.embedLoader.IsSupported(c.Embed.Service) {
			return nil, errors.New("unsupported embed service")
		}
	case *networkv1directory.Listing_Media_:
		if _, err := ppspp.ParseURI(req.Listing.GetMedia().SwarmUri); err != nil {
			return nil, errors.New("invalid swarm uri")
		}
	case *networkv1directory.Listing_Service_:
		// TODO: implement services
	}

	// TODO: moderation

	l, err := newListing(atomic.AddUint64(&d.nextID, 1), req.Listing)
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	l = d.listings.GetOrInsert(l).(*listing)
	s := d.sessions.GetOrInsert(&session{certificate: network.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: network.VPNCertificate(ctx).GetParent()}).(*user)

	l.viewers.Delete(s)
	s.viewedListings.Delete(l)

	l.publishers.ReplaceOrInsert(s)
	s.publishedListings.ReplaceOrInsert(l)
	u.sessions.ReplaceOrInsert(s)

	// HAX
	if media := req.Listing.GetMedia(); media != nil {
		candidate, err := kademlia.UnmarshalID(network.VPNCertificate(ctx).GetKey())
		if err != nil {
			return nil, err
		}
		uri, _ := ppspp.ParseURI(req.Listing.GetMedia().SwarmUri)
		go d.loadMediaEmbed(context.Background(), l.id, uri.ID, candidate)
	}

	return &networkv1directory.PublishResponse{Id: l.id}, nil
}

func (d *directoryService) Unpublish(ctx context.Context, req *networkv1directory.UnpublishRequest) (*networkv1directory.UnpublishResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(req.Id).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s, ok := d.sessions.Get(&session{certificate: network.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}

	if l.publishers.Delete(s) == nil {
		return nil, errors.New("not publishing this listing")
	}
	s.publishedListings.Delete(l)

	d.users.Touch(&user{certificate: network.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.UnpublishResponse{}, nil
}

func (d *directoryService) Join(ctx context.Context, req *networkv1directory.JoinRequest) (*networkv1directory.JoinResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(req.Id).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s := d.sessions.GetOrInsert(&session{certificate: network.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: network.VPNCertificate(ctx).GetParent()}).(*user)

	if !l.publishers.Has(s) {
		l.viewers.ReplaceOrInsert(s)
		s.viewedListings.ReplaceOrInsert(l)
	}
	u.sessions.ReplaceOrInsert(s)

	return &networkv1directory.JoinResponse{}, nil
}

func (d *directoryService) Part(ctx context.Context, req *networkv1directory.PartRequest) (*networkv1directory.PartResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(req.Id).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s, ok := d.sessions.Get(&session{certificate: network.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}

	if l.viewers.Delete(s) == nil {
		return nil, errors.New("not viewing this listing")
	}
	s.viewedListings.Delete(l)

	d.users.Touch(&user{certificate: network.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.PartResponse{}, nil
}

func (d *directoryService) Ping(ctx context.Context, req *networkv1directory.PingRequest) (*networkv1directory.PingResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	s, ok := d.sessions.Get(&session{certificate: network.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}

	s.publishedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		d.listings.Touch(it.(*listing))
		return true
	})
	s.viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		d.listings.Touch(it.(*listing))
		return true
	})

	return &networkv1directory.PingResponse{}, nil
}

func listingKey(m proto.Message) ([]byte, error) {
	opt := proto.MarshalOptions{
		Deterministic: true,
		UseCachedSize: true,
	}
	b := make([]byte, 0, opt.Size(m))
	b, err := opt.MarshalAppend(b, m)
	if err != nil {
		return nil, err
	}

	h, err := blake2b.New256(nil)
	if err != nil {
		return nil, err
	}
	if _, err := h.Write(b); err != nil {
		return nil, err
	}
	return h.Sum(nil), nil
}

func newListing(id uint64, l *networkv1directory.Listing) (*listing, error) {
	key, err := listingKey(l)
	if err != nil {
		return nil, err
	}

	return &listing{
		id:           id,
		key:          key,
		listing:      l,
		nextSnippet:  &networkv1directory.ListingSnippet{},
		snippet:      &networkv1directory.ListingSnippet{},
		modifiedTime: timeutil.Now(),
	}, nil
}

type listing struct {
	id           uint64
	key          []byte
	listing      *networkv1directory.Listing
	nextSnippet  *networkv1directory.ListingSnippet
	snippet      *networkv1directory.ListingSnippet
	modifiedTime timeutil.Time
	publishers   llrb.LLRB
	viewers      llrb.LLRB
}

func (l *listing) ID() uint64 {
	return l.id
}
func (l *listing) Key() []byte {
	return l.key
}

func (l *listing) Less(o llrb.Item) bool {
	return keyerLess(l, o)
}

type session struct {
	certificate       *certificate.Certificate
	publishedListings llrb.LLRB
	viewedListings    llrb.LLRB
}

func (u *session) Key() []byte {
	return u.certificate.Key
}

func (u *session) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}

type user struct {
	certificate *certificate.Certificate
	sessions    llrb.LLRB
}

func (u *user) Key() []byte {
	return u.certificate.Key
}

func (u *user) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}
