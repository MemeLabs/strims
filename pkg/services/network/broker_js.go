// +build js

package network

import (
	"bufio"
	"bytes"
	"context"
	"errors"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
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
		logger: logger,
		broker: NewBroker(logger),
	}
}

// BrokerProxyService provides an RPC service interface for the broker
// factory. The cryptographic operations required for private set intersection
// are expensive. To keep from blocking time sensitive network transfers we
// offload them to a separate web worker.
type BrokerProxyService struct {
	logger     *zap.Logger
	helpers    sync.Map
	nextPeerID uint64
	broker     Broker
}

// Open ...
func (s *BrokerProxyService) Open(ctx context.Context, r *pb.BrokerProxyRequest) (<-chan *pb.BrokerProxyEvent, error) {
	ctx, cancel := context.WithCancel(ctx)

	events := make(chan *pb.BrokerProxyEvent)
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
	s.helpers.Store(pid, h)

	go func() {
		events <- &pb.BrokerProxyEvent{
			Body: &pb.BrokerProxyEvent_Open_{
				Open: &pb.BrokerProxyEvent_Open{
					ProxyId: pid,
				},
			},
		}

		<-ctx.Done()
		s.helpers.Delete(pid)
		rw.Close()
	}()

	return events, nil
}

// SendKeys ...
func (s *BrokerProxyService) SendKeys(ctx context.Context, r *pb.BrokerProxySendKeysRequest) (*pb.BrokerProxySendKeysResponse, error) {
	pi, ok := s.helpers.Load(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	if err := pi.(*brokerProxyServiceHelper).SendKeys(r.Keys); err != nil {
		return nil, err
	}
	return &pb.BrokerProxySendKeysResponse{}, nil
}

// ReceiveKeys ...
func (s *BrokerProxyService) ReceiveKeys(ctx context.Context, r *pb.BrokerProxyReceiveKeysRequest) (*pb.BrokerProxyReceiveKeysResponse, error) {
	pi, ok := s.helpers.Load(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	keys, err := pi.(*brokerProxyServiceHelper).ReceiveKeys(r.Keys)
	if err != nil {
		return nil, err
	}
	return &pb.BrokerProxyReceiveKeysResponse{Keys: keys}, nil
}

// Data ...
func (s *BrokerProxyService) Data(ctx context.Context, r *pb.BrokerProxyDataRequest) (*pb.BrokerProxyDataResponse, error) {
	pi, ok := s.helpers.Load(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	pi.(*brokerProxyServiceHelper).rw.Data(r.Data)
	return &pb.BrokerProxyDataResponse{}, nil
}

// Close ...
func (s *BrokerProxyService) Close(ctx context.Context, r *pb.BrokerProxyCloseRequest) (*pb.BrokerProxyCloseResponse, error) {
	pi, ok := s.helpers.Load(r.ProxyId)
	if !ok {
		return nil, ErrProxyIDNotFound
	}

	pi.(*brokerProxyServiceHelper).Close()
	return &pb.BrokerProxyCloseResponse{}, nil
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
	events   chan *pb.BrokerProxyEvent
	readable chan struct{}
}

func (r *brokerProxyServiceReadWriter) Data(p []byte) {
	r.rb.Write(p)
	r.readable <- struct{}{}
}

func (r *brokerProxyServiceReadWriter) Read(p []byte) (int, error) {
	if r.rb.Len() == 0 {
		r.events <- &pb.BrokerProxyEvent{
			Body: &pb.BrokerProxyEvent_Read_{
				Read: &pb.BrokerProxyEvent_Read{},
			},
		}
		if _, ok := <-r.readable; !ok {
			return 0, ErrProxyClosed
		}
	}
	return r.rb.Read(p)
}

func (r *brokerProxyServiceReadWriter) Write(p []byte) (int, error) {
	r.events <- &pb.BrokerProxyEvent{
		Body: &pb.BrokerProxyEvent_Data_{
			Data: &pb.BrokerProxyEvent_Data{
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
func NewBrokerProxyClient(logger *zap.Logger, bus *wasmio.Bus) *BrokerProxyClient {
	return &BrokerProxyClient{
		logger: logger,
		client: api.NewBrokerProxyClient(rpc.NewClient(logger, bus)),
	}
}

// BrokerProxyClient ...
type BrokerProxyClient struct {
	logger *zap.Logger
	client *api.BrokerProxyClient
}

// SendKeys ...
func (h *BrokerProxyClient) SendKeys(c ReadWriteFlusher, keys [][]byte) error {
	proxy, err := h.proxy(c)
	if err != nil {
		return err
	}
	defer proxy.Close()
	return proxy.SendKeys(keys)
}

// ReceiveKeys ...
func (h *BrokerProxyClient) ReceiveKeys(c ReadWriteFlusher, keys [][]byte) ([][]byte, error) {
	proxy, err := h.proxy(c)
	if err != nil {
		return nil, err
	}
	defer proxy.Close()
	return proxy.ReceiveKeys(keys)
}

func (h *BrokerProxyClient) proxy(conn ReadWriteFlusher) (*brokerProxyClientHelper, error) {
	ctx, cancel := context.WithCancel(context.Background())

	req := &pb.BrokerProxyRequest{ConnMtu: int32(connMTU(conn))}
	events := make(chan *pb.BrokerProxyEvent, 1)
	if err := h.client.Open(ctx, req, events); err != nil {
		cancel()
		return nil, err
	}

	e, ok := <-events
	if !ok {
		cancel()
		return nil, ErrResponseClosed
	}
	o, ok := e.Body.(*pb.BrokerProxyEvent_Open_)
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
	client *api.BrokerProxyClient
	conn   ReadWriteFlusher
	read   chan struct{}
}

func (p *brokerProxyClientHelper) doEventPump(events chan *pb.BrokerProxyEvent) {
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
			case *pb.BrokerProxyEvent_Data_:
				if _, err := p.conn.Write(body.Data.Data); err != nil {
					return
				}
				if err := p.conn.Flush(); err != nil {
					return
				}
			case *pb.BrokerProxyEvent_Read_:
				p.read <- struct{}{}
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

			req := &pb.BrokerProxyDataRequest{
				ProxyId: p.id,
				Data:    b[:n],
			}
			if err := p.client.Data(p.ctx, req, &pb.BrokerProxyDataResponse{}); err != nil {
				return
			}
		}
	}
}

// SendKeys ...
func (p *brokerProxyClientHelper) SendKeys(keys [][]byte) error {
	req := &pb.BrokerProxySendKeysRequest{
		ProxyId: p.id,
		Keys:    keys,
	}
	return p.client.SendKeys(p.ctx, req, &pb.BrokerProxySendKeysResponse{})
}

// ReceiveKeys ...
func (p *brokerProxyClientHelper) ReceiveKeys(keys [][]byte) ([][]byte, error) {
	req := &pb.BrokerProxyReceiveKeysRequest{
		ProxyId: p.id,
		Keys:    keys,
	}
	res := &pb.BrokerProxyReceiveKeysResponse{}
	if err := p.client.ReceiveKeys(p.ctx, req, res); err != nil {
		return nil, err
	}
	return res.Keys, nil
}

func (p *brokerProxyClientHelper) Close() error {
	req := &pb.BrokerProxyCloseRequest{ProxyId: p.id}
	return p.client.Close(p.ctx, req, &pb.BrokerProxyCloseResponse{})
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
