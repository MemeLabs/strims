package directory

import (
	"context"
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dialer"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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
)

// AddressSalt ...
var AddressSalt = []byte("directory")

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

const chunkSize = 128

var swarmOptions = ppspp.SwarmOptions{
	ChunkSize:  chunkSize,
	LiveWindow: 16 * 1024,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
}

func newDirectoryService(logger *zap.Logger, key *key.Key, config *networkv1directory.ServerConfig, ew *protoutil.ChunkStreamWriter) *directoryService {
	return &directoryService{
		logger:          logger,
		done:            make(chan struct{}),
		broadcastTicker: timeutil.DefaultTickEmitter.Ticker(broadcastInterval),
		embedLoadTicker: timeutil.DefaultTickEmitter.Ticker(embedLoadInterval),
		config:          config,
		eventWriter:     ew,
		embedLoader:     newEmbedLoader(logger, config.Integrations),
		listings:        newIndexedLRU(),
	}
}

type directoryService struct {
	logger            *zap.Logger
	transfer          *transfer.Control
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   timeutil.Ticker
	embedLoadTicker   timeutil.Ticker
	lastBroadcastTime timeutil.Time
	config            *networkv1directory.ServerConfig
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
				return err
			}
		case now := <-d.embedLoadTicker.C:
			if err := d.loadEmbeds(ctx, now); err != nil {
				return err
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
		close(d.done)
	})
}

func (d *directoryService) broadcast(now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	var events []*networkv1directory.Event

	for it := d.listings.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
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
						Count: uint32(l.viewers.Len()),
					},
				},
			})
		}
	}

	for it := d.users.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
		u := it.Value().(*user)

		var ids []uint64
		u.sessions.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			it.(*session).viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
				ids = append(ids, it.(*listing).id)
				return true
			})
			return true
		})

		events = append(events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ViewerStateChange_{
				ViewerStateChange: &networkv1directory.Event_ViewerStateChange{
					Subject:    u.certificate.Subject,
					Online:     true,
					ViewingIds: ids,
				},
			},
		})
	}

	eol := now.Add(-sessionTimeout)

	for s, ok := d.sessions.Pop(eol).(*session); ok; s, ok = d.sessions.Pop(eol).(*session) {
		u := d.users.GetOrInsert(&user{certificate: s.certificate.GetParent()}).(*user)
		u.sessions.Delete(s)
		if u.sessions.Len() != 0 {
			continue
		}

		d.users.Delete(u)

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

func (d *directoryService) loadEmbeds(ctx context.Context, now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	embedIDs := map[embedService][]string{}

	d.listings.Each(func(i keyer) bool {
		l := i.(*listing)
		switch c := l.listing.Content.(type) {
		case *networkv1directory.Listing_Embed_:
			svc, _ := toEmbedService(c.Embed.Service)
			embedIDs[svc] = append(embedIDs[svc], c.Embed.Id)
		case *networkv1directory.Listing_Media_:
			uri, _ := ppspp.ParseURI(l.listing.GetMedia().SwarmUri)
			embedIDs[embedServiceSwarm] = append(embedIDs[embedServiceSwarm], uri.ID.String())
		case *networkv1directory.Listing_Service_:
			// TODO: implement services
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

			var snippet *networkv1directory.ListingSnippet
			var ok bool

			switch c := l.listing.Content.(type) {
			case *networkv1directory.Listing_Embed_:
				svc, _ := toEmbedService(c.Embed.Service)
				snippet, ok = res[svc][c.Embed.Id]
			case *networkv1directory.Listing_Media_:
				uri, _ := ppspp.ParseURI(l.listing.GetMedia().SwarmUri)
				snippet, ok = res[embedServiceSwarm][uri.ID.String()]
			case *networkv1directory.Listing_Service_:
				// TODO: implement services
			}

			if ok {
				l.snippet = snippet
				l.modifiedTime = now
				d.listings.Touch(i)
			}
			return true
		})
	}()

	return nil
}

func (d *directoryService) Publish(ctx context.Context, req *networkv1directory.PublishRequest) (*networkv1directory.PublishResponse, error) {
	switch c := req.Listing.Content.(type) {
	case *networkv1directory.Listing_Embed_:
		service, ok := toEmbedService(c.Embed.Service)
		if !ok || !d.embedLoader.IsSupported(service) {
			return nil, errors.New("unsupported embed service")
		}
	case *networkv1directory.Listing_Media_:
		if _, err := ppspp.ParseURI(req.Listing.GetMedia().SwarmUri); err != nil {
			return nil, errors.New("invalid swarm uri")
		}
	case *networkv1directory.Listing_Service_:
		// TODO: implement services
	}

	l, err := newListing(atomic.AddUint64(&d.nextID, 1), req.Listing)
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	l = d.listings.GetOrInsert(l).(*listing)
	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: dialer.VPNCertificate(ctx).GetParent()}).(*user)

	l.publishers.ReplaceOrInsert(s)
	s.publishedListings.ReplaceOrInsert(l)
	u.sessions.ReplaceOrInsert(s)

	return &networkv1directory.PublishResponse{Id: l.id}, nil
}

func (d *directoryService) Unpublish(ctx context.Context, req *networkv1directory.UnpublishRequest) (*networkv1directory.UnpublishResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(req.Id).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s, ok := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}

	if l.publishers.Delete(s) == nil {
		return nil, errors.New("not publishing this listing")
	}
	s.publishedListings.Delete(l)

	return &networkv1directory.UnpublishResponse{}, nil
}

func (d *directoryService) Join(ctx context.Context, req *networkv1directory.JoinRequest) (*networkv1directory.JoinResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.GetByID(req.Id).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: dialer.VPNCertificate(ctx).GetParent()}).(*user)

	l.viewers.ReplaceOrInsert(s)
	s.viewedListings.ReplaceOrInsert(l)
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
	s, ok := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}
	d.users.Touch(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	if l.viewers.Delete(s) == nil {
		return nil, errors.New("not viewing this listing")
	}
	s.viewedListings.Delete(l)

	return &networkv1directory.PartResponse{}, nil
}

func (d *directoryService) Ping(ctx context.Context, req *networkv1directory.PingRequest) (*networkv1directory.PingResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	s, ok := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)}).(*session)
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
		modifiedTime: timeutil.Now(),
	}, nil
}

type listing struct {
	id           uint64
	key          []byte
	listing      *networkv1directory.Listing
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
