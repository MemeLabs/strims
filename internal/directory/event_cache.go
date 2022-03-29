package directory

import (
	"sync"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
)

func newEventCache(network *networkv1.Network) *eventCache {
	return &eventCache{
		Network: network,

		listingChangeEvents:     map[uint64]*networkv1directory.Event_ListingChange{},
		viewerCountChangeEvents: map[uint64]*networkv1directory.Event_ViewerCountChange{},
		viewerStateChangeEvents: map[uint64]*networkv1directory.Event_ViewerStateChange{},
	}
}

type eventCache struct {
	Network *networkv1.Network

	mu                      sync.Mutex
	listingChangeEvents     map[uint64]*networkv1directory.Event_ListingChange
	viewerCountChangeEvents map[uint64]*networkv1directory.Event_ViewerCountChange
	viewerStateChangeEvents map[uint64]*networkv1directory.Event_ViewerStateChange
}

func (d *eventCache) Events() *networkv1directory.EventBroadcast {
	d.mu.Lock()
	defer d.mu.Unlock()

	b := &networkv1directory.EventBroadcast{}
	for _, e := range d.listingChangeEvents {
		b.Events = append(b.Events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ListingChange_{
				ListingChange: e,
			},
		})
	}
	for _, e := range d.viewerCountChangeEvents {
		b.Events = append(b.Events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ViewerCountChange_{
				ViewerCountChange: e,
			},
		})
	}
	for _, e := range d.viewerStateChangeEvents {
		b.Events = append(b.Events, &networkv1directory.Event{
			Body: &networkv1directory.Event_ViewerStateChange_{
				ViewerStateChange: e,
			},
		})
	}
	return b
}

func (d *eventCache) StoreEvent(b *networkv1directory.EventBroadcast) {
	d.mu.Lock()
	defer d.mu.Unlock()

	for _, e := range b.Events {
		switch b := e.Body.(type) {
		case *networkv1directory.Event_ListingChange_:
			d.handleListingChange(b.ListingChange)
		case *networkv1directory.Event_Unpublish_:
			d.handleUnpublish(b.Unpublish)
		case *networkv1directory.Event_ViewerCountChange_:
			d.handleViewerCountChange(b.ViewerCountChange)
		case *networkv1directory.Event_ViewerStateChange_:
			d.handleViewerStateChange(b.ViewerStateChange)
		}
	}
}

func (d *eventCache) handleListingChange(e *networkv1directory.Event_ListingChange) {
	d.listingChangeEvents[e.Id] = e
}

func (d *eventCache) handleUnpublish(e *networkv1directory.Event_Unpublish) {
	delete(d.listingChangeEvents, e.Id)
	delete(d.viewerCountChangeEvents, e.Id)
}

func (d *eventCache) handleViewerCountChange(e *networkv1directory.Event_ViewerCountChange) {
	d.viewerCountChangeEvents[e.Id] = e
}

func (d *eventCache) handleViewerStateChange(e *networkv1directory.Event_ViewerStateChange) {
	if e.Online {
		d.viewerStateChangeEvents[e.Id] = e
	} else {
		delete(d.viewerStateChangeEvents, e.Id)
	}
}
