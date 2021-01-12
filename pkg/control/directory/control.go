package directory

import (
	"bytes"
	"context"
	"errors"
	"strconv"
	"sync"
	"sync/atomic"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
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
	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.NetworkStart:
				t.handleNetworkStart(ctx, e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			}
		case <-ctx.Done():
			return
		}
	}
}

func (t *Control) handleNetworkStart(ctx context.Context, network *network.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.runners.ReplaceOrInsert(newRunner(ctx, t.logger, t.vpn, t.store, t.dialer, t.transfer, network))
}

func (t *Control) handleNetworkStop(network *network.Network) {
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
func (t *Control) ReadEvents(ctx context.Context, networkKey []byte) <-chan *network.DirectoryEvent {
	r, ok := t.runner(networkKey)
	if !ok {
		return nil
	}

	ch := make(chan *network.DirectoryEvent, 128)

	go func() {
		defer close(ch)

	OpenEventReader:
		for {
			er, err := r.EventReader(ctx)
			if err != nil {
				return
			}

			for ctx.Err() == nil {
				event := &network.DirectoryEvent{}
				if err := er.ReadEvent(event); err != nil {
					t.logger.Debug(
						"error reading directory event",
						zap.Error(err),
					)
					continue OpenEventReader
				}

				if event.GetPadding() == nil {
					ch <- event
				}
			}
			return
		}
	}()

	return ch
}

var noopCancelFunc = func() {}

func newRunner(ctx context.Context, logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, dialer *dialer.Control, transfer *transfer.Control, network *network.Network) *runner {
	r := &runner{
		key:     dao.NetworkKey(network),
		network: network,

		logger:   logger,
		vpn:      vpn,
		store:    store,
		dialer:   dialer,
		transfer: transfer,

		runnable:         make(chan bool, 1),
		clientCancelFunc: noopCancelFunc,
		serverCancelFunc: noopCancelFunc,
	}

	r.runnable <- true

	if network.Key != nil {
		go r.tryStartServer(ctx)
	}

	return r
}

type runner struct {
	key     []byte
	network *network.Network

	logger   *zap.Logger
	vpn      *vpn.Host
	store    *dao.ProfileStore
	dialer   *dialer.Control
	transfer *transfer.Control

	lock             sync.Mutex
	closed           bool
	client           *directoryClient
	service          *directoryService
	runnable         chan bool
	clientCancelFunc context.CancelFunc
	serverCancelFunc context.CancelFunc

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
	r.clientCancelFunc()
	r.serverCancelFunc()
}

func (r *runner) Closed() bool {
	r.lock.Lock()
	defer r.lock.Unlock()

	return r.closed
}

func (r *runner) EventReader(ctx context.Context) (*eventReader, error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.service != nil {
		return r.service.eventReader, nil
	}

	r.logger.Info(
		"directory client starting",
		logutil.ByteHex("network", dao.NetworkKey(r.network)),
	)

	<-r.runnable

	client, err := newDirectoryClient(r.logger, r.key, r.transfer)
	if err != nil {
		r.runnable <- true
		return nil, err
	}

	ctx, cancel := context.WithCancel(ctx)

	r.client = client
	r.clientCancelFunc = cancel

	go func() {
		err := client.Run(ctx)
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

	return client.eventReader, nil
}

func (r *runner) closeClient() {
	r.lock.Lock()
	defer r.lock.Unlock()

	r.clientCancelFunc()
}

func (r *runner) tryStartServer(ctx context.Context) {
	for !r.Closed() {
		mu := dao.NewMutex(r.logger, r.store, strconv.AppendUint([]byte("directory:"), r.network.Id, 10))
		muctx, err := mu.Lock(ctx)
		if err != nil {
			return
		}

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
	r.logger.Info(
		"directory server starting",
		logutil.ByteHex("network", dao.NetworkKey(r.network)),
	)

	r.lock.Lock()
	r.clientCancelFunc()

	server, err := r.dialer.Server(r.network.Key.Public, r.network.Key, AddressSalt)
	if err != nil {
		r.lock.Unlock()
		return err
	}

	<-r.runnable

	service, err := newDirectoryService(r.logger, r.network.Key, r.transfer)
	if err != nil {
		r.runnable <- true
		r.lock.Unlock()
		return err
	}

	ctx, cancel := context.WithCancel(ctx)

	r.service = service
	r.serverCancelFunc = cancel
	r.lock.Unlock()

	network.RegisterDirectoryService(server, service)

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error { return service.Run(ctx) })
	eg.Go(func() error { return server.Listen(ctx) })
	err = eg.Wait()

	r.runnable <- true

	r.lock.Lock()
	r.service = nil
	r.lock.Unlock()

	return err
}
