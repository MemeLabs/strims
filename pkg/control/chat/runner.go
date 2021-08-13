package chat

import (
	"bytes"
	"context"
	"errors"
	"strconv"
	"sync"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

func newRunner(ctx context.Context, logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, dialer *dialer.Control, transfer *transfer.Control, key, networkKey []byte, config *chatv1.Server) *runner {
	r := &runner{
		key:        key,
		networkKey: networkKey,
		config:     config,

		logger:   logger.With(logutil.ByteHex("chat", key)),
		vpn:      vpn,
		store:    store,
		dialer:   dialer,
		transfer: transfer,

		runnable: make(chan struct{}, 1),
	}

	r.runnable <- struct{}{}

	if config != nil {
		go r.tryStartServer(ctx)
	}

	return r
}

type runner struct {
	key        []byte
	networkKey []byte
	config     *chatv1.Server

	logger   *zap.Logger
	vpn      *vpn.Host
	store    *dao.ProfileStore
	dialer   *dialer.Control
	transfer *transfer.Control

	lock     sync.Mutex
	closed   bool
	client   *chatReader
	server   *chatServer
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

	r.logger.Info("chat client starting")

	<-r.runnable

	var err error
	r.client, err = newChatReader(r.logger, r.key, r.networkKey)
	if err != nil {
		r.runnable <- struct{}{}
		return nil, err
	}

	go func() {
		err := r.client.Run(ctx, r.transfer)
		r.logger.Debug("chat client closed", zap.Error(err))

		r.runnable <- struct{}{}

		r.lock.Lock()
		r.client = nil
		r.lock.Unlock()
	}()

	return r.client.eventReader, nil
}

func (r *runner) SyncServer(config *chatv1.Server) {
	r.lock.Lock()
	defer r.lock.Unlock()

	r.config = config

	// if r.server != nil {
	// 	r.server.SyncConfig(config)
	// }
}

func (r *runner) tryStartServer(ctx context.Context) {
	for !r.Closed() {
		mu := dao.NewMutex(r.logger, r.store, strconv.AppendUint([]byte("chat:"), r.config.Id, 10))
		muctx, err := mu.Lock(ctx)
		if err != nil {
			return
		}

		r.logger.Info("chat server starting")
		err = r.startServer(muctx)
		r.logger.Info("chat server closed", zap.Error(err))

		mu.Release()
	}
}

func (r *runner) startServer(ctx context.Context) error {
	r.lock.Lock()
	r.client.Close()

	<-r.runnable

	var err error
	r.server, err = newChatServer(r.logger, r.config)
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
