package directory

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/network/dialer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/set"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"golang.org/x/crypto/blake2b"
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

	loadMediaEmbedTimeout = time.Second * 30
	publishQuota          = 10
	viewQuota             = 10
)

var (
	AddressSalt = []byte("directory")
	ConfigSalt  = []byte("directory:config")
)

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

var swarmOptions = ppspp.SwarmOptions{
	ChunkSize:  1024,
	LiveWindow: 2 * 1024,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

var (
	publisherCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_publisher_count",
		Help: "The number of active publishers for each listing",
	}, []string{"type", "id"})
	viewerCount = promauto.NewGaugeVec(prometheus.GaugeOpts{
		Name: "strims_directory_viewer_count",
		Help: "The number of active viewers for each listing",
	}, []string{"type", "id"})
)

func newDirectoryService(
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
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
		done:            make(chan struct{}),
		broadcastTicker: timeutil.DefaultTickEmitter.Ticker(broadcastInterval),
		embedLoadTicker: timeutil.DefaultTickEmitter.Ticker(embedLoadInterval),
		eventWriter:     ew,
		embedLoader:     syncutil.NewPointer(newEmbedLoader(logger, network.GetServerConfig().GetDirectory().GetIntegrations())),
		listings:        newIndexedLRU[listing](),
		listingRecords:  dao.NewDirectoryListingRecordCache(store, nil),
		userRecords:     dao.NewDirectoryUserRecordCache(store, nil),
	}
}

type directoryService struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    network.Dialer

	network           syncutil.Pointer[networkv1.Network]
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   timeutil.Ticker
	embedLoadTicker   timeutil.Ticker
	lastBroadcastTime timeutil.Time
	lastRefreshTime   timeutil.Time
	eventWriter       *protoutil.ChunkStreamWriter
	embedLoader       syncutil.Pointer[embedLoader]
	lock              sync.Mutex
	nextID            uint64
	listings          indexedLRU[listing, *listing]
	sessions          lru[session, *session]
	users             lru[user, *user]
	certificate       *certificate.Certificate
	configPublisher   *vpn.HashTablePublisher
	listingRecords    dao.DirectoryListingRecordCache
	userRecords       dao.DirectoryUserRecordCache
}

func (d *directoryService) Run(ctx context.Context) error {
	defer d.Close()

	if err := d.publishConfig(ctx, d.network.Get()); err != nil {
		return err
	}

	events := make(chan any, 8)
	d.observers.Notify(events)
	defer d.observers.StopNotifying(events)

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *networkv1.NetworkChangeEvent:
				d.handleNetworkChange(e.Network)
				d.publishConfig(ctx, e.Network)
			}
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
		d.userRecords.Close()
		d.listingRecords.Close()
	})
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

func (d *directoryService) publishConfig(ctx context.Context, network *networkv1.Network) error {
	config := network.GetServerConfig().GetDirectory()

	b, err := proto.Marshal(&networkv1directory.ClientConfig{
		Integrations: &networkv1directory.ClientConfig_Integrations{
			Angelthump: config.GetIntegrations().GetAngelthump().GetEnable(),
			Twitch:     config.GetIntegrations().GetTwitch().GetEnable(),
			Youtube:    config.GetIntegrations().GetYoutube().GetEnable(),
			Swarm:      config.GetIntegrations().GetSwarm().GetEnable(),
		},
		PublishQuota:    config.GetPublishQuota(),
		ViewQuota:       config.GetViewQuota(),
		MinPingInterval: config.GetMinPingInterval(),
		MaxPingInterval: config.GetMaxPingInterval(),
	})
	if err != nil {
		return err
	}

	if p := d.configPublisher; p != nil {
		p.Update(b)
		return nil
	}

	n, ok := d.vpn.Node(dao.NetworkKey(network))
	if !ok {
		return errors.New("network not found")
	}
	d.configPublisher, err = n.HashTable.Set(ctx, network.GetServerConfig().GetKey(), ConfigSalt, b)
	return err
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
				Body: &networkv1directory.Event_ViewerCountChange_{
					ViewerCountChange: &networkv1directory.Event_ViewerCountChange{
						Id:    l.id,
						Count: uint32(l.publishers.Len() + l.viewers.Len()),
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
		if (l.publishers.Len() == 0 && l.viewers.Len() == 0) || l.evicted {
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
		events = d.appendUserEvent(events, it.Value())
	}

	eol := now.Add(-sessionTimeout)

	for s := d.sessions.Pop(eol); s != nil; s = d.sessions.Pop(eol) {
		u, userDeleted := d.deleteSession(s)
		if !userDeleted {
			continue
		}

		events = append(events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ViewerStateChange_{
				ViewerStateChange: &networkv1directory.Event_ViewerStateChange{
					Id:      u.id,
					Subject: u.certificate.Subject,
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
	ids := set.New[uint64](8)
	u.EachSession(func(s *session) {
		s.EachViewed(func(l *listing) {
			ids.Insert(l.id)
		})
		s.EachPublished(func(l *listing) {
			ids.Insert(l.id)
		})
	})

	return append(events, &networkv1directory.Event{
		Body: &networkv1directory.Event_ViewerStateChange_{
			ViewerStateChange: &networkv1directory.Event_ViewerStateChange{
				Id:         u.id,
				Subject:    u.certificate.Subject,
				Online:     true,
				ViewingIds: ids.Slice(),
			},
		},
	})
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
	u := d.users.GetOrInsert(&user{certificate: s.certificate.GetParent()})
	u.sessions.Delete(s)

	s.EachViewed(func(l *listing) {
		l.RemoveViewer(s)
		d.listings.Touch(l)
	})
	s.EachPublished(func(l *listing) {
		l.RemovePublisher(s)
		d.listings.Touch(l)
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

	d.listings.Each(func(l *listing) bool {
		if embed := l.listing.GetEmbed(); embed != nil {
			embedIDs[embed.Service] = append(embedIDs[embed.Service], embed.Id)
		}
		return true
	})

	go func() {
		res := d.embedLoader.Get().Load(ctx, embedIDs)
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

func (d *directoryService) loadMediaEmbed(ctx context.Context, listingID uint64, swarmID ppspp.SwarmID, candidate kademlia.ID) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	client, err := d.dialer.ClientWithHostAddr(ctx, d.network.Get().GetServerConfig().GetKey().GetPublic(), candidate, vnic.SnippetPort)
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
		SwarmId: swarmID,
	}
	return snippetClient.Subscribe(ctx, req, ch)
}

func (d *directoryService) getListingRecord(l *networkv1directory.Listing) (*networkv1directory.ListingRecord, error) {
	r, err := d.listingRecords.ByListing.Get(dao.FormatDirectoryListingRecordListingKey(d.network.Get().Id, l))
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, fmt.Errorf("failed to load directory metadata: %w", err)
	}
	return r, nil
}

func (d *directoryService) gettUserRecord(ctx context.Context) (*networkv1directory.UserRecord, error) {
	peerKey := dialer.VPNCertificate(ctx).GetParent().GetKey()
	ur, err := d.userRecords.ByPeerKey.Get(dao.FormatDirectoryUserRecordPeerKeyKey(d.network.Get().Id, peerKey))
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, fmt.Errorf("failed to load user metadata: %w", err)
	}
	return ur, nil
}

func (d *directoryService) Publish(ctx context.Context, req *networkv1directory.PublishRequest) (*networkv1directory.PublishResponse, error) {
	ur, err := d.gettUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if ur.GetModeration().GetDisablePublish().GetValue() {
		return nil, errors.New("publishing from this account is disabled")
	}

	switch c := req.Listing.GetContent().(type) {
	case *networkv1directory.Listing_Embed_:
		if !d.embedLoader.Get().IsSupported(c.Embed.Service) {
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

	l, err := newListing(req.Listing)
	if err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)})
	u := d.users.GetOrInsert(&user{
		id:          ur.Id,
		certificate: dialer.VPNCertificate(ctx).GetParent(),
	})
	if u.PublishedListingCount() >= publishQuota {
		return nil, errors.New("exceeded concurrent publish quota")
	}

	if !d.listings.Has(l) {
		d.nextID++
		l.Init(d.nextID, lr)
	}
	l = d.listings.GetOrInsert(l)

	d.logger.Info(
		"published",
		zap.Object("user", u),
		zap.Object("listing", l),
	)

	l.RemoveViewer(s)
	s.viewedListings.Delete(l)

	l.AddPublisher(s)
	s.publishedListings.ReplaceOrInsert(l)
	u.sessions.ReplaceOrInsert(s)

	// HAX
	if media := req.Listing.GetMedia(); media != nil {
		candidate, err := kademlia.UnmarshalID(dialer.VPNCertificate(ctx).GetKey())
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

	l := d.listings.GetByID(req.Id)
	if l == nil {
		return nil, ErrListingNotFound
	}
	s := d.sessions.Get(&session{certificate: dialer.VPNCertificate(ctx)})
	if s == nil {
		return nil, ErrSessionNotFound
	}

	if !l.RemovePublisher(s) {
		return nil, errors.New("not publishing this listing")
	}
	s.publishedListings.Delete(l)

	d.users.Touch(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.UnpublishResponse{}, nil
}

func (d *directoryService) Join(ctx context.Context, req *networkv1directory.JoinRequest) (*networkv1directory.JoinResponse, error) {
	ur, err := d.gettUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if ur.GetModeration().GetDisableJoin().GetValue() {
		return nil, errors.New("joining from this account is disabled")
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	s := d.sessions.GetOrInsert(&session{certificate: dialer.VPNCertificate(ctx)})
	u := d.users.GetOrInsert(&user{
		id:          ur.Id,
		certificate: dialer.VPNCertificate(ctx).GetParent(),
	})
	if u.ViewedListingCount() >= viewQuota {
		return nil, errors.New("exceeded concurrent view quota")
	}

	l := d.listings.GetByID(req.Id)
	if l == nil {
		return nil, ErrListingNotFound
	}

	d.logger.Info(
		"joined",
		zap.Object("user", u),
		zap.Object("listing", l),
	)

	if !l.publishers.Has(s) {
		l.AddViewer(s)
		s.viewedListings.ReplaceOrInsert(l)
	}
	u.sessions.ReplaceOrInsert(s)

	return &networkv1directory.JoinResponse{}, nil
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

	if !l.RemoveViewer(s) {
		return nil, errors.New("not viewing this listing")
	}
	s.viewedListings.Delete(l)

	d.users.Touch(&user{certificate: dialer.VPNCertificate(ctx).GetParent()})

	return &networkv1directory.PartResponse{}, nil
}

func (d *directoryService) Ping(ctx context.Context, req *networkv1directory.PingRequest) (*networkv1directory.PingResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	ok := d.sessions.Touch(&session{certificate: dialer.VPNCertificate(ctx)})
	if !ok {
		return nil, ErrSessionNotFound
	}

	return &networkv1directory.PingResponse{}, nil
}

func (d *directoryService) ModerateListing(ctx context.Context, req *networkv1directory.ModerateListingRequest) (*networkv1directory.ModerateListingResponse, error) {
	ur, err := d.gettUserRecord(ctx)
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

	key := dao.FormatDirectoryListingRecordListingKey(d.network.Get().Id, l.listing)

	lr, found, err := d.listingRecords.ByListing.GetOrInsert(key, func() (*networkv1directory.ListingRecord, error) {
		return &networkv1directory.ListingRecord{
			NetworkId:  d.network.Get().Id,
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
	d.lock.Unlock()

	l.moderation = lr.Moderation
	l.evicted = lr.Moderation.GetIsBanned().GetValue()

	return &networkv1directory.ModerateListingResponse{}, nil
}

func (d *directoryService) ModerateUser(ctx context.Context, req *networkv1directory.ModerateUserRequest) (*networkv1directory.ModerateUserResponse, error) {
	ur, err := d.gettUserRecord(ctx)
	if err != nil {
		return nil, err
	}
	if !ur.GetModeration().GetIsModerator().GetValue() {
		return nil, errors.New("permission denied")
	}

	key := dao.FormatDirectoryUserRecordPeerKeyKey(d.network.Get().Id, req.PeerKey)

	uur, found, err := d.userRecords.ByPeerKey.GetOrInsert(key, func() (*networkv1directory.UserRecord, error) {
		return &networkv1directory.UserRecord{
			NetworkId:  d.network.Get().Id,
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
	d.lock.Unlock()

	l := d.users.GetByKey(req.PeerKey)
	if l != nil {
		l.EachSession(func(s *session) {
			if uur.GetModeration().GetDisableJoin().GetValue() {
				s.EachViewed(func(l *listing) {
					l.RemoveViewer(s)
					d.listings.Touch(l)
				})
			}
			if uur.GetModeration().GetDisablePublish().GetValue() {
				s.EachPublished(func(l *listing) {
					l.RemovePublisher(s)
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

func newListing(l *networkv1directory.Listing) (*listing, error) {
	key, err := listingKey(l)
	if err != nil {
		return nil, err
	}

	return &listing{
		key:          key,
		listing:      l,
		nextSnippet:  &networkv1directory.ListingSnippet{},
		snippet:      &networkv1directory.ListingSnippet{},
		modifiedTime: timeutil.Now(),
	}, nil
}

type listing struct {
	id             uint64
	rid            uint64
	key            []byte
	listing        *networkv1directory.Listing
	nextSnippet    *networkv1directory.ListingSnippet
	snippet        *networkv1directory.ListingSnippet
	moderation     *networkv1directory.ListingModeration
	modifiedTime   timeutil.Time
	publishers     llrb.LLRB
	viewers        llrb.LLRB
	evicted        bool
	publisherCount prometheus.Gauge
	viewerCount    prometheus.Gauge
}

func (l *listing) Init(id uint64, r *networkv1directory.ListingRecord) {
	l.id = id
	l.moderation = r.GetModeration()
	l.publisherCount = publisherCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
	l.viewerCount = viewerCount.WithLabelValues(listingContentType(l.listing), strconv.FormatUint(id, 10))
}

func (l *listing) Cleanup() {
	publisherCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
	viewerCount.DeleteLabelValues(listingContentType(l.listing), strconv.FormatUint(l.id, 10))
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

func (l *listing) AddViewer(s *session) {
	l.viewers.ReplaceOrInsert(s)
	l.viewerCount.Set(float64(l.viewers.Len()))
}

func (l *listing) RemoveViewer(s *session) bool {
	ok := l.viewers.Delete(s) != nil
	l.viewerCount.Set(float64(l.viewers.Len()))
	return ok
}

func (l *listing) AddPublisher(s *session) {
	l.publishers.ReplaceOrInsert(s)
	l.publisherCount.Set(float64(l.publishers.Len()))
}

func (l *listing) RemovePublisher(s *session) bool {
	ok := l.publishers.Delete(s) != nil
	l.publisherCount.Set(float64(l.publishers.Len()))
	return ok
}

func (l *listing) EachViewer(it func(s *session)) {
	l.viewers.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*session))
		return true
	})
}

func (l *listing) EachPublisher(it func(s *session)) {
	l.publishers.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*session))
		return true
	})
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

	switch c := l.listing.Content.(type) {
	case *networkv1directory.Listing_Media_:
		e.AddString("type", "media")
		e.AddString("uri", c.Media.SwarmUri)
	case *networkv1directory.Listing_Service_:
		e.AddString("type", "service")
		e.AddString("service", c.Service.Type)
	case *networkv1directory.Listing_Embed_:
		e.AddString("type", "embed")
		e.AddString("service", embedServiceName(c.Embed.Service))
		e.AddString("id", c.Embed.GetId())
	case *networkv1directory.Listing_Chat_:
		e.AddString("type", "chat")
		e.AddBinary("key", c.Chat.Key)
		e.AddString("name", c.Chat.Name)
	}
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

func (u *session) Key() []byte {
	return u.certificate.Key
}

func (u *session) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}

type user struct {
	id          uint64
	certificate *certificate.Certificate
	sessions    llrb.LLRB
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

func (u *user) EachSession(it func(s *session)) {
	u.sessions.AscendLessThan(llrb.Inf(1), func(ii llrb.Item) bool {
		it(ii.(*session))
		return true
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
	e.AddString("subject", u.certificate.Subject)
	return nil
}
