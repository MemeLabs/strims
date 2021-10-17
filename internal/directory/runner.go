package directory

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

func newRunner(ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	dialer network.Dialer,
	transfer transfer.Control,
	network *networkv1.Network,
) *runner {
	r := &runner{
		key:     dao.NetworkKey(network),
		network: network,

		logger:   logger.With(logutil.ByteHex("network", dao.NetworkKey(network))),
		vpn:      vpn,
		store:    store,
		dialer:   dialer,
		transfer: transfer,

		runnable: make(chan struct{}, 1),
	}

	r.runnable <- struct{}{}

	if network.GetServerConfig() != nil {
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
	dialer   network.Dialer
	transfer transfer.Control

	lock     sync.Mutex
	closed   bool
	client   *directoryReader
	server   *directoryServer
	runnable chan struct{}
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

func (r *runner) EventReader(ctx context.Context) (*protoutil.ChunkStreamReader, error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.closed {
		return nil, errors.New("cannot read from closed runner")
	}

	if r.server != nil {
		return r.server.eventReader, nil
	}

	r.logger.Info("directory client starting")

	<-r.runnable

	var err error
	r.client, err = newDirectoryReader(r.logger, r.key)
	if err != nil {
		r.runnable <- struct{}{}
		return nil, err
	}

	go func() {
		err := r.client.Run(ctx, r.transfer)
		r.logger.Debug("directory client closed", zap.Error(err))

		r.runnable <- struct{}{}

		r.lock.Lock()
		r.client = nil
		r.lock.Unlock()
	}()

	return r.client.eventReader, nil
}

func (r *runner) tryStartServer(ctx context.Context) {
	for !r.Closed() {
		r.logger.Info("directory server starting")
		err := r.startServer(ctx)
		r.logger.Info("directory server closed", zap.Error(err))

		// mu := dao.NewMutex(r.logger, r.store, strconv.AppendUint([]byte("directory:"), r.network.Id, 10))
		// muctx, err := mu.Lock(ctx)
		// if err != nil {
		// 	return
		// }

		// r.logger.Info("directory server starting")
		// err = r.startServer(muctx)
		// r.logger.Info("directory server closed", zap.Error(err))

		// mu.Release()
	}
}

func (r *runner) startServer(ctx context.Context) error {
	r.lock.Lock()
	r.client.Close()

	<-r.runnable

	var err error
	r.server, err = newDirectoryServer(r.logger, r.network)
	if err != nil {
		r.runnable <- struct{}{}
		r.lock.Unlock()
		return err
	}

	r.lock.Unlock()

	err = r.server.Run(ctx, r.dialer, r.transfer)

	r.lock.Lock()
	r.server = nil
	r.lock.Unlock()

	r.runnable <- struct{}{}

	return err
}
