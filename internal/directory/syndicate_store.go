// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/event"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/logutil"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

type syndicateListing struct {
	Listing
	Viewers map[uint64]*syndicateViewer
}

type syndicateViewer struct {
	User
	Listings map[uint64]*syndicateListing
}

type syndicateStoreObserver[T any] struct {
	ch   chan T
	stop ioutil.Stopper
}

func newSyndicateStore(logger *zap.Logger, network *networkv1.Network) *syndicateStore {
	return &syndicateStore{
		logger:  logger.With(logutil.ByteHex("network", dao.NetworkKey(network))),
		Network: network,

		userObservers:    map[uint64]*event.Observer{},
		listings:         map[uint64]*syndicateListing{},
		viewers:          map[uint64]*syndicateViewer{},
		viewersByPeerKey: hashmap.New[[]byte, *syndicateViewer](hashmap.NewByteInterface[[]byte]()),
	}
}

type syndicateStore struct {
	logger  *zap.Logger
	Network *networkv1.Network

	mu               sync.Mutex
	listingObservers event.Observer
	userObservers    map[uint64]*event.Observer
	listings         map[uint64]*syndicateListing
	viewers          map[uint64]*syndicateViewer
	viewersByPeerKey hashmap.Map[[]byte, *syndicateViewer]
}

func (d *syndicateStore) GetListings() (ls []Listing) {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, l := range d.listings {
		ls = append(ls, l.Listing)
	}
	return
}

func (d *syndicateStore) GetListingByID(id uint64) (Listing, bool) {
	d.mu.Lock()
	defer d.mu.Unlock()

	if l, ok := d.listings[id]; ok {
		return l.Listing, true
	}
	return Listing{}, false
}

func (d *syndicateStore) GetUsers() (us []User) {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, u := range d.viewers {
		us = append(us, u.User)
	}
	return
}

func (d *syndicateStore) GetUsersByListingID(id uint64) (us []User) {
	d.mu.Lock()
	defer d.mu.Unlock()

	if l, ok := d.listings[id]; ok {
		for _, u := range l.Viewers {
			us = append(us, u.User)
		}
	}
	return
}

func (d *syndicateStore) GetViewersByListing(id uint64) {
	// get chat autocomplete list
	// subscribe to updates (viewer adds/removes)
}

func (d *syndicateStore) GetListingsByPeerKey(peerKey []byte) []Listing {
	d.mu.Lock()
	defer d.mu.Unlock()

	var ls []Listing
	if v, ok := d.viewersByPeerKey.Get(peerKey); ok {
		for _, l := range v.Listings {
			ls = append(ls, l.Listing)
		}
	}
	return ls
}

func (d *syndicateStore) HandleEvent(b *networkv1directory.EventBroadcast) {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, e := range b.Events {
		switch b := e.Body.(type) {
		case *networkv1directory.Event_ListingChange_:
			d.handleListingChange(b.ListingChange)
		case *networkv1directory.Event_Unpublish_:
			d.handleUnpublish(b.Unpublish)
		case *networkv1directory.Event_UserCountChange_:
			d.handleUserCountChange(b.UserCountChange)
		case *networkv1directory.Event_UserPresenceChange_:
			d.handleUserPresenceChange(b.UserPresenceChange)
		}
	}
}

func (d *syndicateStore) Reset() {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, l := range d.listings {
		for _, v := range l.Viewers {
			d.emitUserEvent(UserEvent{PartUserEventType, v.User, l.Listing})
		}
		d.listingObservers.Emit(ListingEvent{UnpublishListingEventType, l.Listing})
	}

	d.listings = map[uint64]*syndicateListing{}
	d.viewers = map[uint64]*syndicateViewer{}
	d.viewersByPeerKey = hashmap.New[[]byte, *syndicateViewer](hashmap.NewByteInterface[[]byte]())
}

func (d *syndicateStore) Close() {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, o := range d.userObservers {
		o.Close()
	}
	d.listingObservers.Close()
}

func (d *syndicateStore) handleListingChange(e *networkv1directory.Event_ListingChange) {
	l := d.listings[e.Id]
	if l == nil {
		l = &syndicateListing{
			Listing: Listing{
				ID: e.Id,
			},
			Viewers: map[uint64]*syndicateViewer{},
		}
		d.listings[e.Id] = l
	}

	l.Listing.Listing = e.Listing
	l.Listing.Snippet = e.Snippet
	l.Listing.Moderation = e.Moderation

	d.listingObservers.Emit(ListingEvent{ChangeListingEventType, l.Listing})
}

func (d *syndicateStore) handleUnpublish(e *networkv1directory.Event_Unpublish) {
	l := d.listings[e.Id]
	if l == nil {
		return
	}

	for _, v := range l.Viewers {
		delete(v.Listings, e.Id)
	}
	delete(d.listings, e.Id)

	d.listingObservers.Emit(ListingEvent{UnpublishListingEventType, l.Listing})
}

func (d *syndicateStore) handleUserCountChange(e *networkv1directory.Event_UserCountChange) {
	l := d.listings[e.Id]
	if l == nil {
		return
	}

	l.UserCount = e.UserCount
	l.RecentUserCount = e.RecentUserCount

	d.listingObservers.Emit(ListingEvent{UserCountChangeListingEventType, l.Listing})
}

func (d *syndicateStore) handleUserPresenceChange(e *networkv1directory.Event_UserPresenceChange) {
	v := d.viewers[e.Id]

	if !e.Online {
		if v != nil {
			for _, l := range v.Listings {
				delete(l.Viewers, e.Id)
				d.emitUserEvent(UserEvent{PartUserEventType, v.User, l.Listing})

				d.logger.Debug(
					"parted",
					zap.Object("user", v.User),
					zap.Object("listing", l.Listing),
				)
			}
			delete(d.viewers, e.Id)
			d.viewersByPeerKey.Delete(v.PeerKey)
		}
		return
	}

	if v == nil {
		v = &syndicateViewer{
			User: User{
				ID:      e.Id,
				PeerKey: e.PeerKey,
			},
			Listings: map[uint64]*syndicateListing{},
		}
		d.viewers[e.Id] = v
		d.viewersByPeerKey.Set(v.PeerKey, v)
	}

	if v.User.Alias != e.Alias {
		v.User.Alias = e.Alias

		for _, l := range v.Listings {
			d.emitUserEvent(UserEvent{RenameUserEventType, v.User, l.Listing})
		}

		d.logger.Debug("renamed", zap.Object("user", v.User))
	}

	for _, id := range e.ListingIds {
		if _, ok := v.Listings[id]; !ok {
			if l, ok := d.listings[id]; ok {
				v.Listings[id] = l
				l.Viewers[v.ID] = v
				d.emitUserEvent(UserEvent{JoinUserEventType, v.User, l.Listing})

				d.logger.Debug(
					"joined",
					zap.Object("user", v.User),
					zap.Object("listing", l.Listing),
				)
			}
		}
	}

	for id, l := range v.Listings {
		if !slices.Contains(e.ListingIds, id) {
			delete(v.Listings, id)
			delete(l.Viewers, l.ID)
			d.emitUserEvent(UserEvent{PartUserEventType, v.User, l.Listing})

			d.logger.Debug(
				"parted",
				zap.Object("user", v.User),
				zap.Object("listing", l.Listing),
			)
		}
	}
}

func (d *syndicateStore) NotifyListingEvent(ch chan ListingEvent) {
	d.listingObservers.Notify(ch)
}

func (d *syndicateStore) StopNotifyingListingEvent(ch chan ListingEvent) {
	d.listingObservers.StopNotifying(ch)
}

func (d *syndicateStore) NotifyUserEvent(listingID uint64, ch chan UserEvent) {
	d.mu.Lock()
	defer d.mu.Unlock()

	o, ok := d.userObservers[listingID]
	if !ok {
		o = &event.Observer{}
		d.userObservers[listingID] = o
	}
	o.Notify(ch)
}

func (d *syndicateStore) StopNotifyingUserEvent(listingID uint64, ch chan UserEvent) {
	d.mu.Lock()
	defer d.mu.Unlock()

	o, ok := d.userObservers[listingID]
	if !ok {
		return
	}
	o.StopNotifying(ch)
	if o.Size() == 0 {
		delete(d.userObservers, listingID)
	}
}

func (d *syndicateStore) emitUserEvent(e UserEvent) {
	if o, ok := d.userObservers[e.Listing.ID]; ok {
		o.Emit(e)
	}
}
