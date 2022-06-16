// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
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

type syndicateStoreObserver struct {
	ch   chan UserEvent
	stop ioutil.Stopper
}

func newSyndicateStore(logger *zap.Logger, network *networkv1.Network) *syndicateStore {
	return &syndicateStore{
		logger:  logger.With(logutil.ByteHex("network", dao.NetworkKey(network))),
		Network: network,

		observers:        map[uint64][]syndicateStoreObserver{},
		listings:         map[uint64]*syndicateListing{},
		viewers:          map[uint64]*syndicateViewer{},
		viewersByPeerKey: hashmap.New[[]byte, *syndicateViewer](hashmap.NewByteInterface[[]byte]()),
	}
}

type syndicateStore struct {
	logger  *zap.Logger
	Network *networkv1.Network

	mu               sync.Mutex
	observers        map[uint64][]syndicateStoreObserver
	listings         map[uint64]*syndicateListing
	viewers          map[uint64]*syndicateViewer
	viewersByPeerKey hashmap.Map[[]byte, *syndicateViewer]
}

func (d *syndicateStore) GetListings() (ls []Listing) {
	for _, l := range d.listings {
		ls = append(ls, l.Listing)
	}
	return
}

func (d *syndicateStore) GetListingByID(id uint64) (Listing, bool) {
	if l, ok := d.listings[id]; ok {
		return l.Listing, true
	}
	return Listing{}, false
}

func (d *syndicateStore) GetUsers() (us []User) {
	for _, u := range d.viewers {
		us = append(us, u.User)
	}
	return
}

func (d *syndicateStore) GetUsersByListingID(id uint64) (us []User) {
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
}

func (d *syndicateStore) handleUserCountChange(e *networkv1directory.Event_UserCountChange) {
	l := d.listings[e.Id]
	if l != nil {
		l.UserCount = e.Count
	}
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

func (d *syndicateStore) NotifyUserEvents(listingID uint64, ch chan UserEvent, stop ioutil.Stopper) {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.observers[listingID] = append(d.observers[listingID], syndicateStoreObserver{ch, stop})
}

func (d *syndicateStore) emitUserEvent(e UserEvent) {
	var removed []int
	for i, obs := range d.observers[e.Listing.ID] {
		select {
		case <-obs.stop:
			removed = append(removed, i)
		case obs.ch <- e:
		}
	}

	if len(removed) != 0 {
		obs := d.observers[e.Listing.ID]
		for i, j := range removed {
			copy(obs[j-i:], obs[j-i+1:])
			obs[len(obs)-i-1] = syndicateStoreObserver{}
		}
		if l := len(obs) - len(removed); l == 0 {
			delete(d.observers, e.Listing.ID)
		} else {
			d.observers[e.Listing.ID] = obs[:l]
		}
	}
}
