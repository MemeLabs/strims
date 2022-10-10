// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"math/rand"
	"sync"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/network/dialer"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"golang.org/x/exp/maps"
	"golang.org/x/sync/errgroup"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Listing struct {
	ID              uint64
	Listing         *networkv1directory.Listing
	Snippet         *networkv1directory.ListingSnippet
	Moderation      *networkv1directory.ListingModeration
	UserCount       uint32
	RecentUserCount uint32
}

func (l Listing) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddUint64("id", l.ID)
	marshalListingLogObject(l.Listing, e)
	return nil
}

type NetworkListings struct {
	NetworkKey []byte
	Listings   []Listing
}

type User struct {
	ID      uint64
	Alias   string
	PeerKey []byte
}

func (u User) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddUint64("id", u.ID)
	e.AddBinary("key", u.PeerKey)
	e.AddString("alias", u.Alias)
	return nil
}

type UserEventType int

const (
	_ UserEventType = iota
	JoinUserEventType
	PartUserEventType
	RenameUserEventType
)

type UserEvent struct {
	Type    UserEventType
	User    User
	Listing Listing
}

type ListingEventType int

const (
	_ ListingEventType = iota
	ChangeListingEventType
	UnpublishListingEventType
	UserCountChangeListingEventType
)

type ListingEvent struct {
	Type    ListingEventType
	Listing Listing
}

type AssetBundleEvent struct {
	NetworkID   uint64
	NetworkKey  []byte
	AssetBundle *networkv1directory.AssetBundle
}

type Control interface {
	Run()
	PushSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet)
	DeleteSnippet(swarmID ppspp.SwarmID)
	Publish(ctx context.Context, listing *networkv1directory.Listing, networkKey []byte) (uint64, error)
	Unpublish(ctx context.Context, id uint64, networkKey []byte) error
	Join(ctx context.Context, query *networkv1directory.ListingQuery, networkKey []byte) (uint64, error)
	Part(ctx context.Context, id uint64, networkKey []byte) error
	ModerateListing(ctx context.Context, id uint64, moderation *networkv1directory.ListingModeration, networkKey []byte) error
	ModerateUser(ctx context.Context, peerKey []byte, moderation *networkv1directory.UserModeration, networkKey []byte) error
	GetListingsByPeerKey(peerKey []byte) []NetworkListings
	GetUsersByNetworkID(id uint64) []User
	GetListingsByNetworkID(id uint64) []Listing
	GetListingByQuery(networkID uint64, query *networkv1directory.ListingQuery) (Listing, bool)
	NotifyListingEvent(networkID uint64, ch chan ListingEvent) ([]Listing, error)
	StopNotifyingListingEvent(networkID uint64, ch chan ListingEvent)
	NotifyListingUserEvent(networkID, listingID uint64, ch chan UserEvent) ([]User, error)
	StopNotifyingListingUserEvent(networkID, listingID uint64, ch chan UserEvent)
	WatchAssetBundles(ctx context.Context) <-chan AssetBundleEvent
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store dao.Store,
	observers *event.Observers,
	network network.Control,
	transfer transfer.Control,
) Control {
	snippets := &snippetMap{}

	return &control{
		ctx:       ctx,
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		network:   network,
		transfer:  transfer,
		events:    observers.Chan(),

		runners:               hashmap.New[[]byte, *runner](hashmap.NewByteInterface[[]byte]()),
		syndicateStores:       map[uint64]*syndicateStore{},
		assetBundleEventCache: map[uint64]AssetBundleEvent{},

		snippets: snippets,
		snippetServer: &snippetServer{
			logger:   logger,
			dialer:   network.Dialer(),
			transfer: transfer,
			snippets: snippets,
		},
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	vpn       *vpn.Host
	store     dao.Store
	observers *event.Observers
	network   network.Control
	transfer  transfer.Control

	events                chan any
	lock                  sync.Mutex
	runners               hashmap.Map[[]byte, *runner]
	syndicateStores       map[uint64]*syndicateStore
	assetBundleEventCache map[uint64]AssetBundleEvent

	snippets      *snippetMap
	snippetServer *snippetServer
}

// Run ...
func (t *control) Run() {
	pingTimer := time.NewTimer(pingStartupDelay)
	defer pingTimer.Stop()

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.NetworkStart:
				t.handleNetworkStart(e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			case event.NetworkCertUpdate:
				t.handleNetworkCertUpdate(e.Network)
			case *networkv1.NetworkChangeEvent:
				t.handleNetworkChange(e.Network)
			}
		case <-pingTimer.C:
			fuzz := rand.Int63n(int64((maxPingInterval - minPingInterval)))
			pingTimer.Reset(minPingInterval + time.Duration(fuzz))
			t.ping()
		case <-t.ctx.Done():
			return
		}
	}
}

func (t *control) handleNetworkStart(network *networkv1.Network) {
	logger := t.logger.With(logutil.ByteHex("network", dao.NetworkKey(network)))

	t.lock.Lock()
	defer t.lock.Unlock()

	r, err := newRunner(t.ctx, logger, t.vpn, t.store, t.observers, t.network.Dialer(), t.transfer, network)
	if err != nil {
		logger.Error("failed to start directory runner", zap.Error(err))
		return
	}
	t.runners.Set(dao.NetworkKey(network), r)

	s := newSyndicateStore(logger, network)
	t.syndicateStores[network.Id] = s

	defer t.observers.EmitLocal(event.DirectorySyndicateStart{Network: network})

	go t.snippetServer.start(t.ctx, network)

	go func() {
		defer func() {
			t.lock.Lock()
			delete(t.syndicateStores, network.Id)
			delete(t.assetBundleEventCache, network.Id)
			t.lock.Unlock()

			s.Close()

			t.observers.EmitLocal(event.DirectorySyndicateStop{Network: network})
		}()

		for {
			eg, rctx := errgroup.WithContext(t.ctx)

			readers, stop, err := r.Reader(rctx)
			if err != nil {
				logger.Warn("error getting directory event reader", zap.Error(err))
				return
			}

			eg.Go(func() error {
				for {
					b := &networkv1directory.EventBroadcast{}
					if err := readers.events.Read(b); err != nil {
						return fmt.Errorf("reading event: %w", err)
					} else if rctx.Err() != nil {
						return nil
					}

					s.HandleEvent(b)

					t.observers.EmitLocal(event.DirectoryEvent{
						NetworkID:  network.Id,
						NetworkKey: dao.NetworkKey(network),
						Broadcast:  b,
					})
				}
			})

			eg.Go(func() error {
				for {
					b := &networkv1directory.AssetBundle{}
					if err := readers.assets.Read(b); err != nil {
						return fmt.Errorf("reading asset bundle: %w", err)
					} else if rctx.Err() != nil {
						return nil
					}

					e := AssetBundleEvent{
						NetworkID:   network.Id,
						NetworkKey:  dao.NetworkKey(network),
						AssetBundle: b,
					}

					t.lock.Lock()
					t.assetBundleEventCache[network.Id] = e
					t.lock.Unlock()

					t.observers.EmitLocal(event.DirectoryAssetBundle(e))
				}
			})

			err = eg.Wait()
			done := t.ctx.Err() != nil

			stop()
			s.Reset()

			logger.Info(
				"directory reader closed",
				zap.Error(err),
				zap.Bool("done", done),
			)
			if done {
				return
			}
		}
	}()
}

func (t *control) handleNetworkStop(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.snippetServer.stop(network.Id)

	if r, ok := t.runners.Delete(dao.NetworkKey(network)); ok {
		r.Close()
	}
}

func (t *control) handleNetworkChange(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Get(dao.NetworkKey(network)); ok {
		r.Sync(network)
	}
}

func (t *control) handleNetworkCertUpdate(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Get(dao.NetworkKey(network)); ok {
		go t.pingDirectory(r)
	}
}

func (t *control) ping() {
	t.lock.Lock()
	defer t.lock.Unlock()

	for it := t.runners.Iterate(); it.Next(); {
		go t.pingDirectory(it.Value())
	}
}

func (t *control) pingDirectory(r *runner) {
	c, dc, err := t.client(t.ctx, r.NetworkKey())
	if err != nil {
		r.Logger().Warn("directory ping failed", zap.Error(err))
		return
	}
	defer c.Close()

	err = dc.Ping(t.ctx, &networkv1directory.PingRequest{}, &networkv1directory.PingResponse{})
	if err != nil {
		r.Logger().Warn("directory ping failed", zap.Error(err))
	}
}

func (t *control) client(ctx context.Context, networkKey []byte) (*dialer.RPCClient, *networkv1directory.DirectoryClient, error) {
	client, err := t.network.Dialer().Client(ctx, networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, networkv1directory.NewDirectoryClient(client), nil
}

// PushSnippet ...
func (t *control) PushSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet) {
	t.snippets.Update(swarmID, snippet)
}

// DeleteSnippet ...
func (t *control) DeleteSnippet(swarmID ppspp.SwarmID) {
	t.snippets.Delete(swarmID)
}

// Publish ...
func (t *control) Publish(ctx context.Context, listing *networkv1directory.Listing, networkKey []byte) (uint64, error) {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return 0, err
	}
	defer c.Close()

	res := &networkv1directory.PublishResponse{}
	err = dc.Publish(ctx, &networkv1directory.PublishRequest{Listing: listing}, res)
	return res.Id, err
}

// Unpublish ...
func (t *control) Unpublish(ctx context.Context, id uint64, networkKey []byte) error {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Unpublish(ctx, &networkv1directory.UnpublishRequest{Id: id}, &networkv1directory.UnpublishResponse{})
}

// Join ...
func (t *control) Join(ctx context.Context, query *networkv1directory.ListingQuery, networkKey []byte) (uint64, error) {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return 0, err
	}
	defer c.Close()

	res := &networkv1directory.JoinResponse{}
	err = dc.Join(ctx, &networkv1directory.JoinRequest{Query: query}, res)
	if err != nil {
		return 0, err
	}
	return res.Id, nil
}

// Part ...
func (t *control) Part(ctx context.Context, id uint64, networkKey []byte) error {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Part(ctx, &networkv1directory.PartRequest{Id: id}, &networkv1directory.PartResponse{})
}

func (t *control) ModerateListing(ctx context.Context, id uint64, moderation *networkv1directory.ListingModeration, networkKey []byte) error {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	req := &networkv1directory.ModerateListingRequest{
		Id:         id,
		Moderation: moderation,
	}
	return dc.ModerateListing(ctx, req, &networkv1directory.ModerateListingResponse{})
}

func (t *control) ModerateUser(ctx context.Context, peerKey []byte, moderation *networkv1directory.UserModeration, networkKey []byte) error {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	req := &networkv1directory.ModerateUserRequest{
		PeerKey:    peerKey,
		Moderation: moderation,
	}
	return dc.ModerateUser(ctx, req, &networkv1directory.ModerateUserResponse{})
}

func (t *control) GetListingsByPeerKey(peerKey []byte) []NetworkListings {
	t.lock.Lock()
	stores := maps.Values(t.syndicateStores)
	t.lock.Unlock()

	var nls []NetworkListings
	for _, s := range stores {
		if ls := s.GetListingsByPeerKey(peerKey); len(ls) != 0 {
			nls = append(nls, NetworkListings{
				NetworkKey: dao.NetworkKey(s.Network),
				Listings:   ls,
			})
		}
	}
	return nls
}

func (t *control) GetUsersByNetworkID(id uint64) []User {
	t.lock.Lock()
	s, ok := t.syndicateStores[id]
	t.lock.Unlock()

	if ok {
		return s.GetUsers()
	}
	return nil
}

func (t *control) GetListingsByNetworkID(id uint64) []Listing {
	t.lock.Lock()
	s, ok := t.syndicateStores[id]
	t.lock.Unlock()

	if ok {
		return s.GetListings()
	}
	return nil
}

func (t *control) GetListingByQuery(networkID uint64, query *networkv1directory.ListingQuery) (Listing, bool) {
	t.lock.Lock()
	s, ok := t.syndicateStores[networkID]
	t.lock.Unlock()

	if !ok {
		return Listing{}, false
	}

	switch q := query.Query.(type) {
	case *networkv1directory.ListingQuery_Id:
		return s.GetListingByID(q.Id)
	case *networkv1directory.ListingQuery_Listing:
		key := dao.FormatDirectoryListingRecordListingKey(networkID, q.Listing)
		for _, l := range s.GetListings() {
			if bytes.Equal(key, dao.FormatDirectoryListingRecordListingKey(networkID, l.Listing)) {
				return l, true
			}
		}
	}
	return Listing{}, false
}

func (t *control) NotifyListingEvent(networkID uint64, ch chan ListingEvent) ([]Listing, error) {
	t.lock.Lock()
	s, ok := t.syndicateStores[networkID]
	t.lock.Unlock()
	if !ok {
		return nil, errors.New("network id not found")
	}

	s.NotifyListingEvent(ch)
	return s.GetListings(), nil
}

func (t *control) StopNotifyingListingEvent(networkID uint64, ch chan ListingEvent) {
	t.lock.Lock()
	s, ok := t.syndicateStores[networkID]
	t.lock.Unlock()
	if ok {
		s.StopNotifyingListingEvent(ch)
	}
}

func (t *control) NotifyListingUserEvent(networkID, listingID uint64, ch chan UserEvent) ([]User, error) {
	t.lock.Lock()
	s, ok := t.syndicateStores[networkID]
	t.lock.Unlock()
	if !ok {
		return nil, errors.New("network id not found")
	}

	s.NotifyUserEvent(listingID, ch)
	return s.GetUsersByListingID(listingID), nil
}

func (t *control) StopNotifyingListingUserEvent(networkID, listingID uint64, ch chan UserEvent) {
	t.lock.Lock()
	s, ok := t.syndicateStores[networkID]
	t.lock.Unlock()
	if ok {
		s.StopNotifyingUserEvent(listingID, ch)
	}
}

func (t *control) WatchAssetBundles(ctx context.Context) <-chan AssetBundleEvent {
	ch := make(chan AssetBundleEvent, 8)

	go func() {
		defer close(ch)

		t.lock.Lock()
		es := maps.Values(t.assetBundleEventCache)
		t.lock.Unlock()

		for _, e := range es {
			select {
			case ch <- e:
			case <-ctx.Done():
				return
			}
		}

		events, done := t.observers.Events()
		defer done()

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case event.DirectoryAssetBundle:
					ch <- AssetBundleEvent(e)
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch
}
