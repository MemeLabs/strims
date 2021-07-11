//go:build js
// +build js

package network

import (
	"bufio"
	"bytes"
	"context"
	"errors"
	"sync"
	"sync/atomic"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrProxyIDNotFound = errors.New("proxy id not found")
	ErrProxyClosed     = errors.New("proxy closed")
	ErrResponseClosed  = errors.New("response closed unexpectedly")
	ErrUnexpectedEvent = errors.New("unexpected event type")
)

// NewBrokerProxyService constructs a new BrokerFactoryService
func NewBrokerProxyService(logger *zap.Logger) *BrokerProxyService {
	return &BrokerProxyService{
		logger:  logger,
		helpers: map[uint64]*brokerProxyServiceHelper{},
		broker:  NewBroker(logger),
	}
}

// BrokerProxyService provides an RPC service interface for the broker
// factory. The cryptographic operations required for private set intersection
// are expensive. To keep from blocking time sensitive network transfers we
// offload them to a separate web worker.
type BrokerProxyService struct {
	logger      *zap.Logger
	helpersLock sync.Mutex
	helpers     map[uint64]*brokerProxyServiceHelper
	nextPeerID  uint64
	broker      Broker
}

// Open ...
func (s *BrokerProxyService) Open(ctx context.Context, r *networkv1.BrokerProxyRequest) (<-chan *networkv1.BrokerProxyEvent, error) {
	ctx, cancel := context.WithCancel(ctx)

	events := make(chan *networkv1.BrokerProxyEvent)
	rw := &brokerProxyServiceReadWriter{
		events:   events,
		readable: make(chan struct{}),
	}
	h := &brokerProxyServiceHelper{
		broker: s.broker,
		conn:   bufio.NewReadWriter(bufio.NewReader(rw), bufio.NewWriterSize(rw, int(r.ConnMtu))),
		rw:     rw,
		cancel: cancel,
	}

	pid := atomic.AddUint64(&s.nextPeerID, 1)

	s.helpersLock.Lock()
	s.helpers[pid] = h
	s.helpersLock.Unlock()

	go func() {
		events <- &networkv1.BrokerProxyEvent{
			Body: &networkv1.BrokerProxyEvent_Open_{
				Open: &networkv1.BrokerProxyEvent_Open{
					ProxyId: pid,
				},
			},
		}

		<-ctx.Done()

		s.helpersLock.Lock()
		delete(s.helpers, pid)
		s.helpersLock.Unlock()

		rw.Close()
	}()

	return events, nil
}

func (s *BrokerProxyService) helper(pid uint64) (*brokerProxyServiceHelper, bool) {
	s.helpersLock.Lock()
	pi, ok := s.helpers[pid]
	s.helpersLock.Unlock()
	return pi, ok
}

// SendKeys ...
func (s *BrokerProxyService) SendKeys(ctx context.Context, r *networkv1.BrokerProxySendKeysRequest) (*networkv1.BrokerProxySendKeysResponse, error) {
	pi, ok := s.helper(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	if err := pi.SendKeys(r.Keys); err != nil {
		return nil, err
	}
	return &networkv1.BrokerProxySendKeysResponse{}, nil
}

// ReceiveKeys ...
func (s *BrokerProxyService) ReceiveKeys(ctx context.Context, r *networkv1.BrokerProxyReceiveKeysRequest) (*networkv1.BrokerProxyReceiveKeysResponse, error) {
	pi, ok := s.helper(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	keys, err := pi.ReceiveKeys(r.Keys)
	if err != nil {
		return nil, err
	}
	return &networkv1.BrokerProxyReceiveKeysResponse{Keys: keys}, nil
}

// Data ...
func (s *BrokerProxyService) Data(ctx context.Context, r *networkv1.BrokerProxyDataRequest) (*networkv1.BrokerProxyDataResponse, error) {
	pi, ok := s.helper(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	pi.rw.Data(r.Data)
	return &networkv1.BrokerProxyDataResponse{}, nil
}

// Close ...
func (s *BrokerProxyService) Close(ctx context.Context, r *networkv1.BrokerProxyCloseRequest) (*networkv1.BrokerProxyCloseResponse, error) {
	pi, ok := s.helper(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	pi.Close()
	return &networkv1.BrokerProxyCloseResponse{}, nil
}

type brokerProxyServiceHelper struct {
	broker Broker
	conn   *bufio.ReadWriter
	rw     *brokerProxyServiceReadWriter
	cancel context.CancelFunc
}

// SendKeys ...
func (h *brokerProxyServiceHelper) SendKeys(keys [][]byte) error {
	return h.broker.SendKeys(h.conn, keys)
}

// ReceiveKeys ...
func (h *brokerProxyServiceHelper) ReceiveKeys(keys [][]byte) ([][]byte, error) {
	return h.broker.ReceiveKeys(h.conn, keys)
}

// Close ...
func (h *brokerProxyServiceHelper) Close() {
	h.cancel()
}

type brokerProxyServiceReadWriter struct {
	rb       bytes.Buffer
	events   chan *networkv1.BrokerProxyEvent
	readable chan struct{}
}

func (r *brokerProxyServiceReadWriter) Data(p []byte) {
	r.rb.Write(p)
	r.readable <- struct{}{}
}

func (r *brokerProxyServiceReadWriter) Read(p []byte) (int, error) {
	if r.rb.Len() == 0 {
		r.events <- &networkv1.BrokerProxyEvent{
			Body: &networkv1.BrokerProxyEvent_Read_{
				Read: &networkv1.BrokerProxyEvent_Read{},
			},
		}
		if _, ok := <-r.readable; !ok {
			return 0, ErrProxyClosed
		}
	}
	return r.rb.Read(p)
}

func (r *brokerProxyServiceReadWriter) Write(p []byte) (int, error) {
	r.events <- &networkv1.BrokerProxyEvent{
		Body: &networkv1.BrokerProxyEvent_Data_{
			Data: &networkv1.BrokerProxyEvent_Data{
				Data: append(make([]byte, 0, len(p)), p...),
			},
		},
	}
	return len(p), nil
}

func (r *brokerProxyServiceReadWriter) Close() error {
	close(r.readable)
	return nil
}

// NewBrokerProxyClient ....
func NewBrokerProxyClient(logger *zap.Logger, bus *wasmio.Bus) (*BrokerProxyClient, error) {
	client, err := rpc.NewClient(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: bus,
	})
	if err != nil {
		return nil, err
	}

	return &BrokerProxyClient{
		logger: logger,
		client: networkv1.NewBrokerProxyClient(client),
	}, nil
}

// BrokerProxyClient ...
type BrokerProxyClient struct {
	logger *zap.Logger
	client *networkv1.BrokerProxyClient
}

// SendKeys ...
func (h *BrokerProxyClient) SendKeys(c ioutil.ReadWriteFlusher, keys [][]byte) error {
	proxy, err := h.proxy(c)
	if err != nil {
		return err
	}
	defer proxy.Close()
	return proxy.SendKeys(keys)
}

// ReceiveKeys ...
func (h *BrokerProxyClient) ReceiveKeys(c ioutil.ReadWriteFlusher, keys [][]byte) ([][]byte, error) {
	proxy, err := h.proxy(c)
	if err != nil {
		return nil, err
	}
	defer proxy.Close()
	return proxy.ReceiveKeys(keys)
}

func (h *BrokerProxyClient) proxy(conn ioutil.ReadWriteFlusher) (*brokerProxyClientHelper, error) {
	ctx, cancel := context.WithCancel(context.Background())

	req := &networkv1.BrokerProxyRequest{ConnMtu: int32(connMTU(conn))}
	events := make(chan *networkv1.BrokerProxyEvent, 1)
	go func() {
		if err := h.client.Open(ctx, req, events); err != nil {
			h.logger.Debug("bootstrap proxy client closed", zap.Error(err))
			cancel()
		}
		h.logger.Debug("bootstrap proxy client closed")
	}()

	e, ok := <-events
	if !ok {
		cancel()
		return nil, ErrResponseClosed
	}
	o, ok := e.Body.(*networkv1.BrokerProxyEvent_Open_)
	if !ok {
		cancel()
		return nil, ErrUnexpectedEvent
	}

	p := &brokerProxyClientHelper{
		ctx:    ctx,
		cancel: cancel,
		id:     o.Open.ProxyId,
		client: h.client,
		conn:   conn,
		read:   make(chan struct{}, 1),
	}

	go p.doEventPump(events)
	go p.doReadPump()

	return p, nil
}

type brokerProxyClientHelper struct {
	ctx    context.Context
	cancel context.CancelFunc
	id     uint64
	client *networkv1.BrokerProxyClient
	conn   ioutil.ReadWriteFlusher
	read   chan struct{}
}

func (p *brokerProxyClientHelper) doEventPump(events chan *networkv1.BrokerProxyEvent) {
	defer p.cancel()

	for {
		select {
		case <-p.ctx.Done():
			return
		case e, ok := <-events:
			if !ok {
				return
			}
			switch body := e.Body.(type) {
			case *networkv1.BrokerProxyEvent_Data_:
				if _, err := p.conn.Write(body.Data.Data); err != nil {
					return
				}
				if err := p.conn.Flush(); err != nil {
					return
				}
			case *networkv1.BrokerProxyEvent_Read_:
				select {
				case p.read <- struct{}{}:
				case <-p.ctx.Done():
				}
			}
		}
	}
}

func (p *brokerProxyClientHelper) doReadPump() {
	defer p.cancel()

	b := make([]byte, connMTU(p.conn))
	for {
		select {
		case <-p.ctx.Done():
			return
		case <-p.read:
			n, err := p.conn.Read(b)
			if err != nil {
				return
			}

			req := &networkv1.BrokerProxyDataRequest{
				ProxyId: p.id,
				Data:    b[:n],
			}
			if err := p.client.Data(p.ctx, req, &networkv1.BrokerProxyDataResponse{}); err != nil {
				return
			}
		}
	}
}

// SendKeys ...
func (p *brokerProxyClientHelper) SendKeys(keys [][]byte) error {
	req := &networkv1.BrokerProxySendKeysRequest{
		ProxyId: p.id,
		Keys:    keys,
	}
	return p.client.SendKeys(p.ctx, req, &networkv1.BrokerProxySendKeysResponse{})
}

// ReceiveKeys ...
func (p *brokerProxyClientHelper) ReceiveKeys(keys [][]byte) ([][]byte, error) {
	req := &networkv1.BrokerProxyReceiveKeysRequest{
		ProxyId: p.id,
		Keys:    keys,
	}
	res := &networkv1.BrokerProxyReceiveKeysResponse{}
	if err := p.client.ReceiveKeys(p.ctx, req, res); err != nil {
		return nil, err
	}
	return res.Keys, nil
}

// Close gracefully shuts down the bootstrap service helper allowing any queued
// events to drain.
func (p *brokerProxyClientHelper) Close() error {
	req := &networkv1.BrokerProxyCloseRequest{ProxyId: p.id}
	return p.client.Close(p.ctx, req, &networkv1.BrokerProxyCloseResponse{})
}

const defaultMTU = 16 * 1024

type mtuConn interface {
	MTU() int
}

func connMTU(ci interface{}) int {
	if c, ok := ci.(mtuConn); ok {
		return c.MTU()
	}
	return defaultMTU
}
