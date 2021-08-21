package directory

import (
	"context"
	"errors"
	"math/rand"
	"sync"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

// NewControl ...
func NewControl(logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer control.DialerControl,
	transfer control.TransferControl,
) *Control {
	events := make(chan interface{}, 8)
	observers.Notify(events)

	return &Control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		events:    events,
		dialer:    dialer,
		transfer:  transfer,
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
	events    chan interface{}
	dialer    control.DialerControl
	transfer  control.TransferControl

	lock    sync.Mutex
	runners llrb.LLRB
}

// Run ...
func (t *Control) Run(ctx context.Context) {
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

func (t *Control) handleNetworkStart(ctx context.Context, network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	r := newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, network)
	t.runners.ReplaceOrInsert(r)

	go func() {
		for {
			er, err := r.EventReader(ctx)
			if err != nil {
				t.logger.Debug("error getting directory event reader", zap.Error(err))
				return
			}

			for {
				b := &networkv1.DirectoryEventBroadcast{}
				err := er.Read(b)
				if err == protoutil.ErrShortRead {
					continue
				} else if err != nil {
					t.logger.Debug("error reading directory event", zap.Error(err))
					break
				}

				t.observers.EmitLocal(event.DirectoryEvent{
					NetworkID:  network.Id,
					NetworkKey: dao.GetRootCert(network.Certificate).Key,
					Broadcast:  b,
				})
			}
		}
	}()
}

func (t *Control) handleNetworkStop(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	key := &runner{key: dao.NetworkKey(network)}
	if r, ok := t.runners.Get(key).(*runner); ok {
		t.runners.Delete(key)
		r.Close()
	}
}

func (t *Control) ping(ctx context.Context) {
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
			err := dc.Ping(ctx, &networkv1.DirectoryPingRequest{}, &networkv1.DirectoryPingResponse{})
			if err != nil {
				r.logger.Debug("directory ping failed", zap.Error(err))
			}
			c.Close()
		}()
		return true
	})
}

func (t *Control) client(networkKey []byte) (*rpc.Client, *networkv1.DirectoryClient, error) {
	client, err := t.dialer.Client(networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, nil, err
	}

	return client, networkv1.NewDirectoryClient(client), nil
}

// Publish ...
func (t *Control) Publish(ctx context.Context, listing *networkv1.DirectoryListing, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Publish(ctx, &networkv1.DirectoryPublishRequest{Listing: listing}, &networkv1.DirectoryPublishResponse{})
}

// Unpublish ...
func (t *Control) Unpublish(ctx context.Context, key, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Unpublish(ctx, &networkv1.DirectoryUnpublishRequest{Key: key}, &networkv1.DirectoryUnpublishResponse{})
}

// Join ...
func (t *Control) Join(ctx context.Context, key, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Join(ctx, &networkv1.DirectoryJoinRequest{Key: key}, &networkv1.DirectoryJoinResponse{})
}

// Part ...
func (t *Control) Part(ctx context.Context, key, networkKey []byte) error {
	c, dc, err := t.client(networkKey)
	if err != nil {
		return err
	}
	defer c.Close()

	return dc.Part(ctx, &networkv1.DirectoryPartRequest{Key: key}, &networkv1.DirectoryPartResponse{})
}
