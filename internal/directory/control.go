package directory

import (
	"context"
	"errors"
	"math/rand"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Control interface {
	Run()
	ReadCachedEvents(ctx context.Context, ch chan interface{})
	PushSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet)
	DeleteSnippet(swarmID ppspp.SwarmID)
	Publish(ctx context.Context, listing *networkv1directory.Listing, networkKey []byte) (uint64, error)
	Unpublish(ctx context.Context, id uint64, networkKey []byte) error
	Join(ctx context.Context, id uint64, networkKey []byte) error
	Part(ctx context.Context, id uint64, networkKey []byte) error
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

		eventCache: map[uint64]*eventCache{},

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

	events     chan interface{}
	lock       sync.Mutex
	runners    llrb.LLRB
	eventCache map[uint64]*eventCache

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
	t.runners.ReplaceOrInsert(r)

	c := newEventCache(network)
	t.eventCache[network.Id] = c

	go t.snippetServer.start(t.ctx, network)

	go func() {
		defer func() {
			t.lock.Lock()
			defer t.lock.Unlock()
			delete(t.eventCache, network.Id)
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

				c.StoreEvent(b)
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

	if r, ok := t.runners.Delete(&runner{key: dao.NetworkKey(network)}).(*runner); ok {
		r.Close()
	}
}

func (t *control) handleNetworkChange(network *networkv1.Network) {
	if r, ok := t.runners.Get(&runner{key: dao.NetworkKey(network)}).(*runner); ok {
		r.Sync(network)
	}
}

func (t *control) ping() {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.runners.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		r := i.(*runner)
		c, dc, err := t.client(t.ctx, r.key)
		if err != nil {
			r.Logger().Debug("directory ping failed", zap.Error(err))
			return true
		}

		go func() {
			err := dc.Ping(t.ctx, &networkv1directory.PingRequest{}, &networkv1directory.PingResponse{})
			if err != nil {
				r.Logger().Debug("directory ping failed", zap.Error(err))
			}
			c.Close()
		}()
		return true
	})
}

func (t *control) client(ctx context.Context, networkKey []byte) (*network.RPCClient, *networkv1directory.DirectoryClient, error) {
	client, err := t.network.Dialer().Client(ctx, networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, networkv1directory.NewDirectoryClient(client), nil
}

func (t *control) ReadCachedEvents(ctx context.Context, ch chan interface{}) {
	var es []event.DirectoryEvent
	t.lock.Lock()
	for _, c := range t.eventCache {
		es = append(es, event.DirectoryEvent{
			NetworkID:  c.Network.Id,
			NetworkKey: dao.CertificateRoot(c.Network.Certificate).Key,
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
func (t *control) Join(ctx context.Context, id uint64, networkKey []byte) error {
	c, dc, err := t.client(ctx, networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Join(ctx, &networkv1directory.JoinRequest{Id: id}, &networkv1directory.JoinResponse{})
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
