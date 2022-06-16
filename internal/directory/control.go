// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"bytes"
	"context"
	"errors"
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
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Listing struct {
	ID         uint64
	Listing    *networkv1directory.Listing
	Snippet    *networkv1directory.ListingSnippet
	Moderation *networkv1directory.ListingModeration
	UserCount  uint32
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

type Control interface {
	Run()
	ReadCachedEvents(ctx context.Context, ch chan any)
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
	WatchListingUsers(ctx context.Context, networkID, listingID uint64) ([]User, chan UserEvent, error)
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
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

		runners:         hashmap.New[[]byte, *runner](hashmap.NewByteInterface[[]byte]()),
		eventCache:      map[uint64]*eventCache{},
		syndicateStores: map[uint64]*syndicateStore{},

		snippets: snippets,
		snippetServer: &snippetServer{
			logger:   logger,
			dialer:   network.Dialer(),
			transfer: transfer,
			snippets: snippets,
			servers:  map[uint64]context.CancelFunc{},
		},
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	network   network.Control
	transfer  transfer.Control

	events          chan any
	lock            sync.Mutex
	runners         hashmap.Map[[]byte, *runner]
	eventCache      map[uint64]*eventCache
	syndicateStores map[uint64]*syndicateStore

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
	t.lock.Lock()
	defer t.lock.Unlock()

	r, err := newRunner(t.ctx, t.logger, t.vpn, t.store, t.observers, t.network.Dialer(), t.transfer, network)
	if err != nil {
		t.logger.Error("failed to start directory runner", zap.Error(err))
		return
	}
	t.runners.Set(dao.NetworkKey(network), r)

	c := newEventCache(network)
	t.eventCache[network.Id] = c
	s := newSyndicateStore(t.logger, network)
	t.syndicateStores[network.Id] = s

	go t.snippetServer.start(t.ctx, network)

	go func() {
		defer func() {
			t.lock.Lock()
			defer t.lock.Unlock()
			delete(t.eventCache, network.Id)
			delete(t.syndicateStores, network.Id)
		}()

		for {
			er, stop, err := r.Reader(t.ctx)
			if err != nil {
				t.logger.Debug("error getting directory event reader", zap.Error(err))
				return
			}

			for {
				b := &networkv1directory.EventBroadcast{}
				if err := er.Read(b); err != nil {
					t.logger.Debug("error reading directory event", zap.Error(err))
					break
				}

				t.lock.Lock()
				c.StoreEvent(b)
				s.HandleEvent(b) // TODO: syncutil map?
				t.lock.Unlock()

				t.observers.EmitLocal(event.DirectoryEvent{
					NetworkID:  network.Id,
					NetworkKey: dao.NetworkKey(network),
					Broadcast:  b,
				})
			}
			stop()
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

func (t *control) ping() {
	t.lock.Lock()
	defer t.lock.Unlock()

	for it := t.runners.Iterate(); it.Next(); {
		r := it.Value()

		c, dc, err := t.client(t.ctx, r.NetworkKey())
		if err != nil {
			r.Logger().Debug("directory ping failed", zap.Error(err))
			continue
		}

		go func() {
			err := dc.Ping(t.ctx, &networkv1directory.PingRequest{}, &networkv1directory.PingResponse{})
			if err != nil {
				r.Logger().Debug("directory ping failed", zap.Error(err))
			}
			c.Close()
		}()
	}
}

func (t *control) client(ctx context.Context, networkKey []byte) (*dialer.RPCClient, *networkv1directory.DirectoryClient, error) {
	client, err := t.network.Dialer().Client(ctx, networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, networkv1directory.NewDirectoryClient(client), nil
}

func (t *control) ReadCachedEvents(ctx context.Context, ch chan any) {
	var es []event.DirectoryEvent
	t.lock.Lock()
	for _, c := range t.eventCache {
		es = append(es, event.DirectoryEvent{
			NetworkID:  c.Network.Id,
			NetworkKey: dao.NetworkKey(c.Network),
			Broadcast:  c.Events(),
		})
	}
	t.lock.Unlock()

	for _, e := range es {
		select {
		case ch <- e:
		case <-ctx.Done():
			return
		}
	}
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
	defer t.lock.Unlock()

	var nls []NetworkListings
	for _, s := range t.syndicateStores {
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
	defer t.lock.Unlock()

	if s, ok := t.syndicateStores[id]; ok {
		return s.GetUsers()
	}
	return nil
}

func (t *control) GetListingsByNetworkID(id uint64) []Listing {
	t.lock.Lock()
	defer t.lock.Unlock()

	if s, ok := t.syndicateStores[id]; ok {
		return s.GetListings()
	}
	return nil
}

func (t *control) GetListingByQuery(networkID uint64, query *networkv1directory.ListingQuery) (Listing, bool) {
	t.lock.Lock()
	defer t.lock.Unlock()

	s, ok := t.syndicateStores[networkID]
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

func (t *control) WatchListingUsers(ctx context.Context, networkID, listingID uint64) ([]User, chan UserEvent, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	s, ok := t.syndicateStores[networkID]
	if !ok {
		return nil, nil, errors.New("network id not found")
	}

	users := s.GetUsersByListingID(listingID)

	ch := make(chan UserEvent, 1)
	s.NotifyUserEvents(listingID, ch, ctx.Done())

	return users, ch, nil
}
