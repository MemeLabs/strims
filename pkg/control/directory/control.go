package directory

import (
	"bytes"
	"context"
	"errors"
	"math/rand"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrNetworkNotFound = errors.New("network not found")
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers, dialer *dialer.Control, transfer *transfer.Control) *Control {
	events := make(chan interface{}, 128)
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
	dialer    *dialer.Control
	transfer  *transfer.Control

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

	t.runners.ReplaceOrInsert(newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, network))
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

func (t *Control) runner(networkKey []byte) (*runner, bool) {
	t.lock.Lock()
	defer t.lock.Unlock()

	r, ok := t.runners.Get(&runner{key: networkKey}).(*runner)
	return r, ok
}

// ReadEvents ...
func (t *Control) ReadEvents(ctx context.Context, networkKey []byte) <-chan *networkv1.DirectoryEvent {
	r, ok := t.runner(networkKey)
	if !ok {
		return nil
	}

	ch := make(chan *networkv1.DirectoryEvent, 128)

	go func() {
		defer close(ch)

	OpenEventReader:
		for {
			er, err := r.EventReader(ctx)
			if err != nil {
				t.logger.Debug(
					"error getting directory event reader",
					zap.Error(err),
				)
				return
			}

			for ctx.Err() == nil {
				b := &networkv1.DirectoryEventBroadcast{}
				if err := er.Read(b); err != nil {
					t.logger.Debug(
						"error reading directory event",
						zap.Error(err),
					)
					continue OpenEventReader
				}
				for _, e := range b.Events {
					ch <- e
				}
			}
			return
		}
	}()

	return ch
}

func (t *Control) ping(ctx context.Context) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.runners.AscendLessThan(llrb.Inf(1), func(i llrb.Item) bool {
		client, err := t.client(i.(*runner).key)
		if err != nil {
			t.logger.Debug(
				"pinging directory failed",
				logutil.ByteHex("network", i.(*runner).key),
				zap.Error(err),
			)
			return true
		}

		go client.Ping(ctx, &networkv1.DirectoryPingRequest{}, &networkv1.DirectoryPingResponse{})
		return true
	})
}

func (t *Control) client(networkKey []byte) (*networkv1.DirectoryClient, error) {
	client, err := t.dialer.Client(networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, err
	}

	return networkv1.NewDirectoryClient(client), nil
}

// Publish ...
func (t *Control) Publish(ctx context.Context, listing *networkv1.DirectoryListing, networkKey []byte) error {
	client, err := t.client(networkKey)
	if err != nil {
		return err
	}

	return client.Publish(ctx, &networkv1.DirectoryPublishRequest{Listing: listing}, &networkv1.DirectoryPublishResponse{})
}

// Unpublish ...
func (t *Control) Unpublish(ctx context.Context, key, networkKey []byte) error {
	client, err := t.client(networkKey)
	if err != nil {
		return err
	}

	return client.Unpublish(ctx, &networkv1.DirectoryUnpublishRequest{Key: key}, &networkv1.DirectoryUnpublishResponse{})
}

// Join ...
func (t *Control) Join(ctx context.Context, key, networkKey []byte) error {
	client, err := t.client(networkKey)
	if err != nil {
		return err
	}

	return client.Join(ctx, &networkv1.DirectoryJoinRequest{Key: key}, &networkv1.DirectoryJoinResponse{})
}

// Part ...
func (t *Control) Part(ctx context.Context, key, networkKey []byte) error {
	client, err := t.client(networkKey)
	if err != nil {
		return err
	}

	return client.Part(ctx, &networkv1.DirectoryPartRequest{Key: key}, &networkv1.DirectoryPartResponse{})
}

var noopCancelFunc = func() {}

func newRunner(ctx context.Context, logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, dialer *dialer.Control, transfer *transfer.Control, network *networkv1.Network) *runner {
	r := &runner{
		key:     dao.NetworkKey(network),
		network: network,

		logger:   logger,
		vpn:      vpn,
		store:    store,
		dialer:   dialer,
		transfer: transfer,

		runnable: make(chan bool, 1),
	}

	r.runnable <- true

	if network.Key != nil {
		go r.tryStartServer(ctx)
	}

	return r
}

type runner struct {
	key     []byte
	network *networkv1.Network

	logger   *zap.Logger
	vpn      *vpn.Host
	store    *dao.ProfileStore
	dialer   *dialer.Control
	transfer *transfer.Control

	lock     sync.Mutex
	closed   bool
	client   *directoryReader
	server   *directoryServer
	runnable chan bool

	meme atomic.Value
}

func (r *runner) Less(o llrb.Item) bool {
	if o, ok := o.(*runner); ok {
		return bytes.Compare(r.key, o.key) == -1
	}
	return !o.Less(r)
}

func (r *runner) Close() {
	r.lock.Lock()
	defer r.lock.Unlock()

	r.closed = true
	r.client.Close()
	r.server.Close()
}

func (r *runner) Closed() bool {
	r.lock.Lock()
	defer r.lock.Unlock()

	return r.closed
}

func (r *runner) EventReader(ctx context.Context) (*EventReader, error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.server != nil {
		return r.server.eventReader, nil
	}

	r.logger.Info(
		"directory client starting",
		logutil.ByteHex("network", dao.NetworkKey(r.network)),
	)

	<-r.runnable

	var err error
	r.client, err = newDirectoryReader(r.logger, r.key)
	if err != nil {
		r.runnable <- true
		return nil, err
	}

	go func() {
		err := r.client.Run(ctx, r.transfer)
		r.logger.Debug(
			"directory client closed",
			logutil.ByteHex("network", dao.NetworkKey(r.network)),
			zap.Error(err),
		)

		r.runnable <- true

		r.lock.Lock()
		r.client = nil
		r.lock.Unlock()
	}()

	return r.client.eventReader, nil
}

func (r *runner) tryStartServer(ctx context.Context) {
	for !r.Closed() {
		mu := dao.NewMutex(r.logger, r.store, strconv.AppendUint([]byte("directory:"), r.network.Id, 10))
		muctx, err := mu.Lock(ctx)
		if err != nil {
			return
		}

		r.logger.Info(
			"directory server starting",
			logutil.ByteHex("network", dao.NetworkKey(r.network)),
		)
		err = r.startServer(muctx)
		r.logger.Info(
			"directory server closed",
			logutil.ByteHex("network", dao.NetworkKey(r.network)),
			zap.Error(err),
		)

		mu.Release()
	}
}

func (r *runner) startServer(ctx context.Context) error {
	r.lock.Lock()
	r.client.Close()

	<-r.runnable

	var err error
	r.server, err = newDirectoryServer(r.logger, r.network)
	if err != nil {
		r.runnable <- true
		r.lock.Unlock()
		return err
	}

	r.lock.Unlock()

	err = r.server.Run(ctx, r.dialer, r.transfer)

	r.lock.Lock()
	r.server = nil
	r.lock.Unlock()

	r.runnable <- true

	return err
}
