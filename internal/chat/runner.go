package chat

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
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
	key []byte,
	networkKey []byte,
	config *chatv1.Server,
) *runner {
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
	dialer   network.Dialer
	transfer transfer.Control

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

func (r *runner) Readers(ctx context.Context) (events, assets *protoutil.ChunkStreamReader, err error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.closed {
		return nil, nil, errors.New("cannot read from closed runner")
	}

	if r.server != nil {
		events, assets = r.server.Readers(ctx)
		return
	}

	r.logger.Info("chat client starting")

	<-r.runnable

	r.client, err = newChatReader(r.logger, r.key, r.networkKey)
	if err != nil {
		r.runnable <- struct{}{}
		return nil, nil, err
	}

	go func() {
		err := r.client.Run(ctx, r.transfer)
		r.logger.Debug("chat client closed", zap.Error(err))

		r.runnable <- struct{}{}

		r.lock.Lock()
		r.client = nil
		r.lock.Unlock()
	}()

	events, assets = r.client.Readers(ctx)
	return
}

func (r *runner) SyncServer() {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.server != nil {
		r.server.Sync()
	}
}

func (r *runner) tryStartServer(ctx context.Context) {
	for !r.Closed() {
		mu := dao.NewMutex(r.logger, r.store, r.config.Id)
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
	r.server, err = newChatServer(r.logger, r.store, r.config)
	if err != nil {
		r.runnable <- struct{}{}
		r.lock.Unlock()
		return err
	}

	r.lock.Unlock()

	publisher := newRunnerDirectoryPublisher(r)
	go func() {
		if err := publisher.publish(ctx); err != nil {
			r.logger.Info("publishing chat server to directory failed", zap.Error(err))
		}
	}()

	err = r.server.Run(ctx, r.dialer, r.transfer)

	r.lock.Lock()
	r.server = nil
	r.lock.Unlock()

	go func() {
		if err := publisher.unpublish(); err != nil {
			r.logger.Info("unpublishing chat server from directory failed", zap.Error(err))
		}
	}()

	r.runnable <- struct{}{}

	return err
}

func newRunnerDirectoryPublisher(r *runner) *runnerDirectoryPublisher {
	return &runnerDirectoryPublisher{
		r:    r,
		done: make(chan struct{}),
	}
}

type runnerDirectoryPublisher struct {
	r    *runner
	done chan struct{}
	id   uint64
}

func (p *runnerDirectoryPublisher) client(ctx context.Context) (*network.RPCClient, error) {
	return p.r.dialer.Client(ctx, p.r.networkKey, p.r.networkKey, directory.AddressSalt)
}

func (p *runnerDirectoryPublisher) publish(ctx context.Context) error {
	defer close(p.done)

	client, err := p.client(ctx)
	if err != nil {
		return err
	}

	req := &networkv1directory.PublishRequest{
		Listing: &networkv1directory.Listing{
			Content: &networkv1directory.Listing_Chat_{
				Chat: &networkv1directory.Listing_Chat{
					Key:  p.r.config.Key.Public,
					Name: p.r.config.Room.Name,
				},
			},
		},
	}
	res := &networkv1directory.PublishResponse{}
	if err := networkv1directory.NewDirectoryClient(client).Publish(ctx, req, res); err != nil {
		return err
	}

	p.id = res.Id
	return nil
}

func (p *runnerDirectoryPublisher) unpublish() error {
	<-p.done
	if p.id == 0 {
		return errors.New("directory listing id not set")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	client, err := p.client(ctx)
	if err != nil {
		return err
	}

	return networkv1directory.NewDirectoryClient(client).Unpublish(
		ctx,
		&networkv1directory.UnpublishRequest{Id: p.id},
		&networkv1directory.UnpublishResponse{},
	)
}
