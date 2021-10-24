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
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

type Control interface {
	Run(ctx context.Context)
	PushSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet)
	DeleteSnippet(swarmID ppspp.SwarmID)
	Publish(ctx context.Context, listing *networkv1directory.Listing, networkKey []byte) (uint64, error)
	Unpublish(ctx context.Context, id uint64, networkKey []byte) error
	Join(ctx context.Context, id uint64, networkKey []byte) error
	Part(ctx context.Context, id uint64, networkKey []byte) error
}

// NewControl ...
func NewControl(logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	network network.Control,
	transfer transfer.Control,
) Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	snippets := &snippetMap{}

	return &control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
		network:   network,
		transfer:  transfer,

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
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	events    chan interface{}
	network   network.Control
	transfer  transfer.Control

	lock    sync.Mutex
	runners llrb.LLRB

	snippets      *snippetMap
	snippetServer *snippetServer
}

// Run ...
func (t *control) Run(ctx context.Context) {
	pingTimer := time.NewTimer(pingStartupDelay)
	defer pingTimer.Stop()

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.NetworkStart:
				t.handleNetworkStart(ctx, e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			}
		case <-pingTimer.C:
			fuzz := rand.Int63n(int64((maxPingInterval - minPingInterval)))
			pingTimer.Reset(minPingInterval + time.Duration(fuzz))
			t.ping(ctx)
		case <-ctx.Done():
			return
		}
	}
}

func (t *control) handleNetworkStart(ctx context.Context, network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	r := newRunner(ctx, t.logger, t.vpn, t.store, t.network.Dialer(), t.transfer, network)
	t.runners.ReplaceOrInsert(r)

	go t.snippetServer.start(ctx, network)

	go func() {
		for {
			er, err := r.EventReader(ctx)
			if err != nil {
				t.logger.Debug("error getting directory event reader", zap.Error(err))
				return
			}

			for {
				b := &networkv1directory.EventBroadcast{}
				err := er.Read(b)
				if err == protoutil.ErrShortRead {
					continue
				} else if err != nil {
					t.logger.Debug("error reading directory event", zap.Error(err))
					break
				}

				t.observers.EmitLocal(event.DirectoryEvent{
					NetworkID:  network.Id,
					NetworkKey: dao.CertificateRoot(network.Certificate).Key,
					Broadcast:  b,
				})
			}
		}
	}()
}

func (t *control) handleNetworkStop(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.snippetServer.stop(network.Id)

	key := &runner{key: dao.NetworkKey(network)}
	if r, ok := t.runners.Get(key).(*runner); ok {
		t.runners.Delete(key)
		r.Close()
	}
}

func (t *control) ping(ctx context.Context) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.runners.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		r := i.(*runner)
		c, dc, err := t.client(r.key)
		if err != nil {
			r.logger.Debug("directory ping failed", zap.Error(err))
			return true
		}

		go func() {
			err := dc.Ping(ctx, &networkv1directory.PingRequest{}, &networkv1directory.PingResponse{})
			if err != nil {
				r.logger.Debug("directory ping failed", zap.Error(err))
			}
			c.Close()
		}()
		return true
	})
}

func (t *control) client(networkKey []byte) (*network.RPCClient, *networkv1directory.DirectoryClient, error) {
	client, err := t.network.Dialer().Client(networkKey, networkKey, AddressSalt)
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
	c, dc, err := t.client(networkKey)
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
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Unpublish(ctx, &networkv1directory.UnpublishRequest{Id: id}, &networkv1directory.UnpublishResponse{})
}

// Join ...
func (t *control) Join(ctx context.Context, id uint64, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Join(ctx, &networkv1directory.JoinRequest{Id: id}, &networkv1directory.JoinResponse{})
}

// Part ...
func (t *control) Part(ctx context.Context, id uint64, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Part(ctx, &networkv1directory.PartRequest{Id: id}, &networkv1directory.PartResponse{})
}
