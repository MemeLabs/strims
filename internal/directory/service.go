// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/network/dialer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/sortutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"golang.org/x/exp/slices"
	"google.golang.org/protobuf/proto"
)

const (
	broadcastInterval = time.Second
	sessionTimeout    = time.Minute * 15
	pingStartupDelay  = time.Second * 30
	minPingInterval   = time.Minute * 10
	maxPingInterval   = time.Minute * 14
	embedLoadInterval = time.Minute
	refreshInterval   = time.Minute * 5
	recentUserTTL     = time.Minute * 30

	loadMediaEmbedTimeout = time.Second * 30
	publishQuota          = 10
	viewQuota             = 10
)

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

var (
	publisherSessionCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_publisher_count",
		Help: "The number of active publisher sessions for each listing",
	}, []string{"type", "id"})
	viewerSessionCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_viewer_count",
		Help: "The number of active viewer sessions for each listing",
	}, []string{"type", "id"})
	publisherUserCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_publisher_user_count",
		Help: "The number of active publisher users for each listing",
	}, []string{"type", "id"})
	viewerUserCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_viewer_user_count",
		Help: "The number of active viewer users for each listing",
	}, []string{"type", "id"})
)

func newDirectoryService(
	logger *zap.Logger,
	vpn *vpn.Host,
	store dao.Store,
	observers *event.Observers,
	dialer network.Dialer,
	network *networkv1.Network,
	ew *protoutil.ChunkStreamWriter,
) *directoryService {
	return &directoryService{
		logger:          logger,
		vpn:             vpn,
		store:           store,
		observers:       observers,
		dialer:          dialer,
		network:         syncutil.NewPointer(network),
		broadcastTicker: timeutil.DefaultTickEmitter.Ticker(broadcastInterval),
		embedLoadTicker: timeutil.DefaultTickEmitter.Ticker(embedLoadInterval),
		eventWriter:     ew,
		embedLoader:     syncutil.NewPointer(newEmbedLoader(logger, network.GetServerConfig().GetDirectory().GetIntegrations())),
		listings:        newIndexedLRU[listing](),
		users:           newIndexedLRU[user](),
		listingRecords:  dao.NewDirectoryListingRecordCache(store, nil),
		userRecords:     dao.NewDirectoryUserRecordCache(store, nil),
	}
}

type directoryService struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     dao.Store
	observers *event.Observers
	dialer    network.Dialer

	network           atomic.Pointer[networkv1.Network]
	broadcastTicker   timeutil.Ticker
	embedLoadTicker   timeutil.Ticker
	lastBroadcastTime timeutil.Time
	lastRefreshTime   timeutil.Time
	eventWriter       *protoutil.ChunkStreamWriter
	embedLoader       atomic.Pointer[embedLoader]
	lock              sync.Mutex
	nextID            uint64
	listings          indexedLRU[listing, *listing]
	sessions          lru[session, *session]
	users             indexedLRU[user, *user]
	certificate       *certificate.Certificate
	configPublisher   *vpn.HashTablePublisher
	listingRecords    dao.DirectoryListingRecordCache
	userRecords       dao.DirectoryUserRecordCache
}

func (d *directoryService) Run(ctx context.Context) error {
	defer d.Close()

	events, done := d.observers.Events()
	defer done()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *networkv1.NetworkChangeEvent:
				d.handleNetworkChange(e.Network)
			}
		case now := <-d.broadcastTicker.C:
			if err := d.broadcast(now); err != nil {
				d.logger.Debug("directory broadcast failed", zap.Error(err))
			}
		case now := <-d.embedLoadTicker.C:
			if err := d.loadEmbeds(ctx, now); err != nil {
				d.logger.Debug("directory embed loading failed", zap.Error(err))
			}
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func (d *directoryService) Close() {
	d.broadcastTicker.Stop()
	d.embedLoadTicker.Stop()
	d.userRecords.Close()
	d.listingRecords.Close()
}

func (d *directoryService) handleNetworkChange(network *networkv1.Network) {
	d.network.Swap(network)

	config := network.GetServerConfig().GetDirectory()

	loader := newEmbedLoader(d.logger, config.GetIntegrations())
	d.embedLoader.Swap(loader)

	d.lock.Lock()
	defer d.lock.Unlock()

	d.listings.Each(func(l *listing) bool {
		if e := l.listing.GetEmbed(); e != nil && !loader.IsSupported(e.Service) {
			l.evicted = true
			d.listings.Touch(l)
		}
		return true
	})
}

func (d *directoryService) broadcast(now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	var events []*networkv1directory.Event

	if d.lastRefreshTime.Add(refreshInterval).Before(now) {
		for it := d.listings.IterateTouchedBefore(d.lastBroadcastTime); it.Next(); {
			l := it.Value()
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_ListingChange_{
					ListingChange: &networkv1directory.Event_ListingChange{
						Id:         l.id,
						Listing:    l.listing,
						Snippet:    l.snippet,
						Moderation: l.moderation,
					},
				},
			})
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_UserCountChange_{
					UserCountChange: &networkv1directory.Event_UserCountChange{
						Id:              l.id,
						UserCount:       l.userCount,
						RecentUserCount: uint32(l.RecentUserCount(now.Add(-recentUserTTL))),
					},
				},
			})
		}

		for it := d.users.IterateTouchedBefore(d.lastBroadcastTime); it.Next(); {
			events = d.appendUserEvent(events, it.Value())
		}

		d.lastRefreshTime = now
	}

	for it := d.listings.IterateTouchedAfter(d.lastBroadcastTime); it.Next(); {
		l := it.Value()
		if l.publisherSessions.Len() == 0 || l.evicted {
			d.deleteListing(l)
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
						Id:         l.id,
						Listing:    l.listing,
						Snippet:    l.snippet,
						Moderation: l.moderation,
					},
				},
			})
		} else {
			events = append(events, &networkv1directory.Event{
				Body: &networkv1directory.Event_UserCountChange_{
					UserCountChange: &networkv1directory.Event_UserCountChange{
						Id:              l.id,
						UserCount:       l.userCount,
						RecentUserCount: uint32(l.RecentUserCount(now.Add(-recentUserTTL))),
					},
				},
			})
		}
	}

	for it := d.users.IterateTouchedAfter(d.lastBroadcastTime); it.Next(); {
		events = d.appendUserEvent(events, it.Value())
	}

	eol := now.Add(-sessionTimeout)

	for s := d.sessions.Pop(eol); s != nil; s = d.sessions.Pop(eol) {
		u, userDeleted := d.deleteSession(s)
		if !userDeleted {
			continue
		}

		events = append(events, &networkv1directory.Event{
			Body: &networkv1directory.Event_UserPresenceChange_{
				UserPresenceChange: &networkv1directory.Event_UserPresenceChange{
					Id:      u.id,
					Alias:   u.certificate.Subject,
					PeerKey: u.certificate.Key,
					Online:  false,
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
	return append(events, &networkv1directory.Event{
		Body: &networkv1directory.Event_UserPresenceChange_{
			UserPresenceChange: &networkv1directory.Event_UserPresenceChange{
				Id:         u.id,
				Alias:      u.certificate.Subject,
				PeerKey:    u.certificate.Key,
				Online:     true,
				ListingIds: u.ListingIDs(),
			},
		},
	})
}

func (d *directoryService) upsertUser(q *user) *user {
	u := d.users.GetOrInsert(q)
	u.Update(q)
	return u
}

func (d *directoryService) deleteListing(l *listing) {
	d.listings.Delete(l)

	l.EachViewer(func(s *session) {
		s.viewedListings.Delete(l)
		d.users.Touch(&user{certificate: s.certificate.GetParent()})
	})
	l.EachPublisher(func(s *session) {
		s.publishedListings.Delete(l)
		d.users.Touch(&user{certificate: s.certificate.GetParent()})
	})

	l.Cleanup()
}

func (d *directoryService) deleteSession(s *session) (*user, bool) {
	u := d.upsertUser(&user{certificate: s.certificate.GetParent()})
	u.sessions.Delete(s)

	s.EachViewed(func(l *listing) {
		d.removeListingViewer(l, u, s)
		d.listings.Touch(l)
	})
	s.EachPublished(func(l *listing) {
		d.removeListingPublisher(l, u, s)
		d.listings.Touch(l)
	})

	if u.sessions.Len() != 0 {
		return u, false
	}
	d.users.Delete(u)
	return u, true
}

func (d *directoryService) addListingViewer(l *listing, u *user, s *session) {
	l.viewerSessions.ReplaceOrInsert(s)
	l.viewerSessionCount.Set(float64(l.viewerSessions.Len()))

	l.recentUsers.GetOrInsert(u)

	if !l.viewerUsers.Has(u) {
		l.viewerUsers.ReplaceOrInsert(u)
		l.viewerUserCount.Set(float64(l.viewerUsers.Len()))
		if !l.publisherUsers.Has(u) {
			l.userCount++
		}
	}
	s.viewedListings.ReplaceOrInsert(l)
}

func (d *directoryService) removeListingViewer(l *listing, u *user, s *session) bool {
	ok := l.viewerSessions.Delete(s) != nil
	if !ok {
		return false
	}

	l.viewerSessionCount.Set(float64(l.viewerSessions.Len()))

	s.viewedListings.Delete(l)
	if !u.HasViewedListing(l) {
		ok := l.viewerUsers.Delete(u) != nil
		l.viewerUserCount.Set(float64(l.viewerUsers.Len()))
		if ok && !u.HasPublishedListing(l) {
			l.userCount--
		}
	}

	return true
}

func (d *directoryService) addListingPublisher(l *listing, u *user, s *session) {
	l.publisherSessions.ReplaceOrInsert(s)
	l.publisherSessionCount.Set(float64(l.publisherSessions.Len()))

	l.recentUsers.GetOrInsert(u)

	if !l.publisherUsers.Has(u) {
		l.publisherUsers.ReplaceOrInsert(u)
		l.publisherUserCount.Set(float64(l.publisherUsers.Len()))
		if !l.viewerUsers.Has(u) {
			l.userCount++
		}
	}
	s.publishedListings.ReplaceOrInsert(l)
}

func (d *directoryService) removeListingPublisher(l *listing, u *user, s *session) bool {
	ok := l.publisherSessions.Delete(s) != nil
	if !ok {
		return false
	}

	l.publisherSessionCount.Set(float64(l.publisherSessions.Len()))

	s.publishedListings.Delete(l)
	if !u.HasPublishedListing(l) {
		ok := l.publisherUsers.Delete(u) != nil
		l.publisherUserCount.Set(float64(l.publisherUsers.Len()))
		if ok && !u.HasViewedListing(l) {
			l.userCount--
		}
	}

	return true
}

func (d *directoryService) loadEmbeds(ctx context.Context, now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	embedIDs := map[networkv1directory.Listing_Embed_Service][]string{}

	d.listings.Each(func(l *listing) bool {
		if embed := l.listing.GetEmbed(); embed != nil {
			embedIDs[embed.Service] = append(embedIDs[embed.Service], embed.Id)
		}
		return true
	})

	go func() {
		res := d.embedLoader.Load().Load(ctx, embedIDs)
		now := timeutil.Now()

		d.lock.Lock()
		defer d.lock.Unlock()

		d.listings.Each(func(l *listing) bool {
			if embed := l.listing.GetEmbed(); embed != nil {
				if svcSnippets, ok := res[embed.Service]; ok {
					if snippet, ok := svcSnippets[embed.Id]; ok {
						l.snippet = snippet
						l.modifiedTime = now
						d.listings.Touch(l)
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

	l := d.listings.GetByID(listingID)
	if l == nil {
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

func (d *directoryService) loadMediaEmbed(stop ioutil.Stopper, listingID uint64, swarmID ppspp.SwarmID, candidate kademlia.ID) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := d.dialer.ClientWithHostAddr(ctx, d.network.Load().GetServerConfig().GetKey().GetPublic(), candidate, vnic.SnippetPort)
	if err != nil {
		return nil
	}
	defer client.Close()

	snippetClient := networkv1directory.NewDirectorySnippetClient(client)

	ch := make(chan *networkv1directory.SnippetSubscribeResponse, 16)
	go func() {
		defer cancel()
		for {
			select {
			case res, ok := <-ch:
				if !ok || res.SnippetDelta == nil {
					return
				}
				if d.mergeSnippet(listingID, res.SnippetDelta) {
					// TODO: move on to another candidate if we haven't received a
					// successful update after... some timeout
				}
			case <-stop:
				return
			}
		}
	}()

	req := &networkv1directory.SnippetSubscribeRequest{
		SwarmId: swarmID,
	}
	return snippetClient.Subscribe(ctx, req, ch)
}

func (d *directoryService) getListingRecord(l *networkv1directory.Listing) (*networkv1directory.ListingRecord, error) {
	r, err := d.listingRecords.ByListing.Get(dao.FormatDirectoryListingRecordListingKey(d.network.Load().Id, l))
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, fmt.Errorf("failed to load directory metadata: %w", err)
	}
	return r, nil
}

func (d *directoryService) getUserRecord(ctx context.Context) (*networkv1directory.UserRecord, error) {
	peerKey := dialer.VPNCertificate(ctx).GetParent().GetKey()
	ur, err := d.userRecords.ByPeerKey.Get(dao.FormatDirectoryUserRecordPeerKeyKey(d.network.Load().Id, peerKey))
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, fmt.Errorf("failed to load user metadata: %w", err)
	}
	return ur, nil
}

func (d *directoryService) getListingByQuery(l *networkv1directory.ListingQuery) *listing {
	switch q := l.Query.(type) {
	case *networkv1directory.ListingQuery_Id:
		return d.listings.GetByID(q.Id)
	case *networkv1directory.ListingQuery_Listing:
		return d.listings.GetByKey(formatListingKey(q.Listing))
	default:
		return nil
	}
}

func (d *directoryService) Publish(ctx context.Context, req *networkv1directory.PublishRequest) (*networkv1directory.PublishResponse, error) {
	ur, err := d.getUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if ur.GetModeration().GetDisablePublish().GetValue() {
		return nil, errors.New("publishing from this account is disabled")
	}

	switch c := req.Listing.GetContent().(type) {
	case *networkv1directory.Listing_Embed_:
		if !d.embedLoader.Load().IsSupported(c.Embed.Service) {
			return nil, errors.New("unsupported embed service")
		}
	case *networkv1directory.Listing_Media_:
		if _, err := ppspp.ParseURI(req.Listing.GetMedia().SwarmUri); err != nil {
			return nil, errors.New("invalid swarm uri")
		}
	case *networkv1directory.Listing_Chat_:
		// TODO: validation...
	case *networkv1directory.Listing_Service_:
		// TODO: ingress publishing, chat, ca, etc...
	default:
		return nil, errors.New("unsupported content type")
	}

	lr, err := d.getListingRecord(req.Listing)
	if err != nil {
		return nil, err
	}
	if lr.GetModeration().GetIsBanned().GetValue() {
		return nil, errors.New("listing banned")
	}

	l, err := newListing(req.Listing, lr)
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)})
	u := d.upsertUser(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})
	if u.PublishedListingCount() >= publishQuota {
		return nil, errors.New("exceeded concurrent publish quota")
	}

	l = d.listings.GetOrInsert(l)

	d.logger.Info(
		"published",
		zap.Object("user", u),
		zap.Object("listing", l),
	)

	d.removeListingViewer(l, u, s)
	d.addListingPublisher(l, u, s)

	u.sessions.ReplaceOrInsert(s)

	// HAX
	if media := req.Listing.GetMedia(); media != nil {
		candidate, err := kademlia.UnmarshalID(dialer.VPNCertificate(ctx).GetKey())
		if err != nil {
			return nil, err
		}
		uri, _ := ppspp.ParseURI(req.Listing.GetMedia().SwarmUri)
		go d.loadMediaEmbed(l.Done(), l.id, uri.ID, candidate)
	}

	return &networkv1directory.PublishResponse{Id: l.id}, nil
}

func (d *directoryService) Unpublish(ctx context.Context, req *networkv1directory.UnpublishRequest) (*networkv1directory.UnpublishResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l := d.listings.GetByID(req.Id)
	if l == nil {
		return nil, ErrListingNotFound
	}
	s := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)})
	if s == nil {
		return nil, ErrSessionNotFound
	}
	u := d.upsertUser(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	if !d.removeListingPublisher(l, u, s) {
		return nil, errors.New("not publishing this listing")
	}

	d.users.Touch(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.UnpublishResponse{}, nil
}

func (d *directoryService) Join(ctx context.Context, req *networkv1directory.JoinRequest) (*networkv1directory.JoinResponse, error) {
	ur, err := d.getUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if ur.GetModeration().GetDisableJoin().GetValue() {
		return nil, errors.New("joining from this account is disabled")
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)})
	u := d.upsertUser(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})
	if u.ViewedListingCount() >= viewQuota {
		return nil, errors.New("exceeded concurrent view quota")
	}

	l := d.getListingByQuery(req.Query)
	if l == nil {
		return nil, ErrListingNotFound
	}

	d.logger.Info(
		"joined",
		zap.Object("user", u),
		zap.Object("listing", l),
	)

	if !l.publisherSessions.Has(s) {
		d.addListingViewer(l, u, s)
	}
	u.sessions.ReplaceOrInsert(s)

	return &networkv1directory.JoinResponse{Id: l.id}, nil
}

func (d *directoryService) Part(ctx context.Context, req *networkv1directory.PartRequest) (*networkv1directory.PartResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l := d.listings.GetByID(req.Id)
	if l == nil {
		return nil, ErrListingNotFound
	}
	s := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)})
	if s == nil {
		return nil, ErrSessionNotFound
	}

	u := d.upsertUser(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	if !d.removeListingViewer(l, u, s) {
		return nil, errors.New("not viewing this listing")
	}

	d.users.Touch(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.PartResponse{}, nil
}

func (d *directoryService) Ping(ctx context.Context, req *networkv1directory.PingRequest) (*networkv1directory.PingResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	s := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)})
	if s == nil {
		return nil, ErrSessionNotFound
	}

	u := d.upsertUser(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	s.EachViewed(func(l *listing) {
		l.recentUsers.Touch(u)
	})
	s.EachPublished(func(l *listing) {
		l.recentUsers.Touch(u)
	})

	return &networkv1directory.PingResponse{}, nil
}

func (d *directoryService) ModerateListing(ctx context.Context, req *networkv1directory.ModerateListingRequest) (*networkv1directory.ModerateListingResponse, error) {
	ur, err := d.getUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if !ur.GetModeration().GetIsModerator().GetValue() {
		return nil, errors.New("permission denied")
	}

	d.lock.Lock()
	l := d.listings.GetByID(req.Id)
	d.lock.Unlock()
	if l == nil {
		return nil, ErrListingNotFound
	}

	key := dao.FormatDirectoryListingRecordListingKey(d.network.Load().Id, l.listing)

	lr, found, err := d.listingRecords.ByListing.GetOrInsert(key, func() (*networkv1directory.ListingRecord, error) {
		return &networkv1directory.ListingRecord{
			NetworkId:  d.network.Load().Id,
			Listing:    l.listing,
			Moderation: req.Moderation,
		}, nil
	})
	if found {
		lr, err = d.listingRecords.ByListing.Transform(key, func(m *networkv1directory.ListingRecord) error {
			if v := req.Moderation.GetIsMature(); v != nil {
				m.Moderation.IsMature = v
			}
			if v := req.Moderation.GetIsBanned(); v != nil {
				m.Moderation.IsBanned = v
			}
			if v := req.Moderation.GetCategory(); v != nil {
				m.Moderation.Category = v
			}
			return nil
		})
	}
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	l.moderation = lr.Moderation
	l.evicted = lr.Moderation.GetIsBanned().GetValue()

	return &networkv1directory.ModerateListingResponse{}, nil
}

func (d *directoryService) ModerateUser(ctx context.Context, req *networkv1directory.ModerateUserRequest) (*networkv1directory.ModerateUserResponse, error) {
	ur, err := d.getUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if !ur.GetModeration().GetIsModerator().GetValue() {
		return nil, errors.New("permission denied")
	}

	key := dao.FormatDirectoryUserRecordPeerKeyKey(d.network.Load().Id, req.PeerKey)

	uur, found, err := d.userRecords.ByPeerKey.GetOrInsert(key, func() (*networkv1directory.UserRecord, error) {
		return &networkv1directory.UserRecord{
			NetworkId:  d.network.Load().Id,
			PeerKey:    req.PeerKey,
			Moderation: req.Moderation,
		}, nil
	})
	if found {
		uur, err = d.userRecords.ByPeerKey.Transform(key, func(m *networkv1directory.UserRecord) error {
			if v := req.Moderation.GetDisableJoin(); v != nil {
				m.Moderation.DisableJoin = v
			}
			if v := req.Moderation.GetDisablePublish(); v != nil {
				m.Moderation.DisablePublish = v
			}

			if ur.Moderation.GetIsAdmin().GetValue() {
				if v := req.Moderation.GetIsModerator(); v != nil {
					m.Moderation.IsModerator = v
				}
			}
			return nil
		})
	}
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	u := d.users.GetByKey(req.PeerKey)
	if u != nil {
		u.EachSession(func(s *session) {
			if uur.GetModeration().GetDisableJoin().GetValue() {
				s.EachViewed(func(l *listing) {
					d.removeListingViewer(l, u, s)
					d.listings.Touch(l)
				})
			}
			if uur.GetModeration().GetDisablePublish().GetValue() {
				s.EachPublished(func(l *listing) {
					d.removeListingPublisher(l, u, s)
					d.listings.Touch(l)
				})
			}
		})
	}

	return &networkv1directory.ModerateUserResponse{}, nil
}

func embedServiceName(s networkv1directory.Listing_Embed_Service) string {
	switch s {
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP:
		return "angelthump"
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM:
		return "twitch"
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD:
		return "twitch-vod"
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE:
		return "youtube"
	default:
		return "unknown"
	}
}

func formatListingKey(l *networkv1directory.Listing) []byte {
	return dao.FormatDirectoryListingRecordListingKey(0, l)
}

func newListing(l *networkv1directory.Listing, r *networkv1directory.ListingRecord) (*listing, error) {
	return &listing{
		key:          formatListingKey(l),
		listing:      l,
		nextSnippet:  &networkv1directory.ListingSnippet{},
		snippet:      &networkv1directory.ListingSnippet{},
		moderation:   r.GetModeration(),
		modifiedTime: timeutil.Now(),
		done:         make(chan struct{}),
	}, nil
}

type listing struct {
	id                    uint64
	rid                   uint64
	key                   []byte
	listing               *networkv1directory.Listing
	nextSnippet           *networkv1directory.ListingSnippet
	snippet               *networkv1directory.ListingSnippet
	moderation            *networkv1directory.ListingModeration
	userCount             uint32
	recentUsers           lru[user, *user]
	modifiedTime          timeutil.Time
	publisherSessions     llrb.LLRB
	viewerSessions        llrb.LLRB
	publisherUsers        llrb.LLRB
	viewerUsers           llrb.LLRB
	evicted               bool
	publisherSessionCount prometheus.Gauge
	viewerSessionCount    prometheus.Gauge
	publisherUserCount    prometheus.Gauge
	viewerUserCount       prometheus.Gauge
	done                  chan struct{}
}

func (l *listing) InitID(id uint64) {
	l.id = id
	l.publisherSessionCount = publisherSessionCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
	l.viewerSessionCount = viewerSessionCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
	l.publisherUserCount = publisherUserCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
	l.viewerUserCount = viewerUserCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
}

func (l *listing) Cleanup() {
	publisherSessionCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
	viewerSessionCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
	publisherUserCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
	viewerUserCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
	close(l.done)
}

func (l *listing) Done() <-chan struct{} {
	return l.done
}

func listingContentType(l *networkv1directory.Listing) string {
	switch c := l.Content.(type) {
	case *networkv1directory.Listing_Media_:
		return "media"
	case *networkv1directory.Listing_Service_:
		return "service"
	case *networkv1directory.Listing_Embed_:
		return embedServiceName(c.Embed.Service)
	case *networkv1directory.Listing_Chat_:
		return "chat"
	default:
		return "unknown"
	}
}

func (l *listing) EachViewer(it func(s *session)) {
	l.viewerSessions.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*session))
		return true
	})
}

func (l *listing) EachPublisher(it func(s *session)) {
	l.publisherSessions.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*session))
		return true
	})
}

func (l *listing) RecentUserCount(eol timeutil.Time) int {
	for l.recentUsers.Pop(eol) != nil {
	}
	return l.recentUsers.Len()
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

func (l *listing) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddUint64("id", l.id)
	marshalListingLogObject(l.listing, e)
	return nil
}

type session struct {
	certificate       *certificate.Certificate
	publishedListings llrb.LLRB
	viewedListings    llrb.LLRB
}

func (s *session) EachViewed(it func(l *listing)) {
	s.viewedListings.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*listing))
		return true
	})
}

func (s *session) EachPublished(it func(l *listing)) {
	s.publishedListings.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*listing))
		return true
	})
}

func (s *session) Key() []byte {
	return s.certificate.Key
}

func (s *session) Less(o llrb.Item) bool {
	return keyerLess(s, o)
}

type user struct {
	id          uint64
	certificate *certificate.Certificate
	sessions    llrb.LLRB
}

func (u *user) InitID(id uint64) {
	u.id = id
}

func (u *user) Update(o *user) {
	if !proto.Equal(u.certificate, o.certificate) {
		u.certificate = o.certificate
	}
}

func (u *user) PublishedListingCount() int {
	var n int
	u.EachSession(func(s *session) {
		n += s.publishedListings.Len()
	})
	return n
}

func (u *user) ViewedListingCount() int {
	var n int
	u.EachSession(func(s *session) {
		n += s.viewedListings.Len()
	})
	return n
}

func (u *user) ListingCount() int {
	var n int
	u.EachSession(func(s *session) {
		n += s.publishedListings.Len()
		n += s.viewedListings.Len()
	})
	return n
}

func (u *user) ListingIDs() []uint64 {
	ids := make([]uint64, 0, u.ListingCount())
	u.EachSession(func(s *session) {
		s.EachViewed(func(l *listing) {
			ids = append(ids, l.id)
		})
		s.EachPublished(func(l *listing) {
			ids = append(ids, l.id)
		})
	})

	sortutil.Uint64(ids)
	return slices.Compact(ids)
}

func (u *user) HasViewedListing(l *listing) bool {
	var v bool
	u.eachSession(func(s *session) bool {
		v = s.viewedListings.Has(l)
		return !v
	})
	return v
}

func (u *user) HasPublishedListing(l *listing) bool {
	var v bool
	u.eachSession(func(s *session) bool {
		v = s.publishedListings.Has(l)
		return !v
	})
	return v
}

func (u *user) EachSession(it func(s *session)) {
	u.eachSession(func(s *session) bool {
		it(s)
		return true
	})
}

func (u *user) eachSession(it func(s *session) bool) {
	u.sessions.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		return it(ii.(*session))
	})
}

func (u *user) ID() uint64 {
	return u.id
}

func (u *user) Key() []byte {
	return u.certificate.Key
}

func (u *user) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}

func (u *user) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddBinary("key", u.certificate.Key)
	e.AddString("alias", u.certificate.Subject)
	return nil
}
