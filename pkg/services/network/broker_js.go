// +build js

package network

import (
	"bufio"
	"context"
	"errors"
	"io"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
)

var errPeeerIDNotFound = errors.New("peer id not found")

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
	peers      sync.Map
	nextPeerID uint64
	broker     Broker
}

// Open ...
func (s *BrokerProxyService) Open(ctx context.Context, r *pb.BrokerProxyRequest) (<-chan *pb.BrokerProxyEvent, error) {
	ch := make(chan *pb.BrokerProxyEvent, 1)

	r0, w0 := io.Pipe()
	r1, w1 := io.Pipe()

	p := &brokerProxyServiceHelper{
		broker: s.broker,
		conn:   bufio.NewReadWriter(bufio.NewReader(r0), bufio.NewWriterSize(w1, int(r.ConnMtu))),
		w:      w0,
	}

	pid := atomic.AddUint64(&s.nextPeerID, 1)
	s.peers.Store(pid, p)

	ch <- &pb.BrokerProxyEvent{
		Body: &pb.BrokerProxyEvent_Open_{
			Open: &pb.BrokerProxyEvent_Open{
				PeerId: pid,
			},
		},
	}

	go func() {
		defer func() {
			s.peers.Delete(pid)
			close(ch)
		}()

		b := make([]byte, r.ConnMtu)
		for {
			n, err := r1.Read(b)
			if err != nil {
				return
			}

			ch <- &pb.BrokerProxyEvent{
				Body: &pb.BrokerProxyEvent_Data_{
					Data: &pb.BrokerProxyEvent_Data{
						Data: b[:n],
					},
				},
			}
		}
	}()

	return ch, nil
}

// SendKeys ...
func (s *BrokerProxyService) SendKeys(ctx context.Context, req *pb.BrokerProxySendKeysRequest) (*pb.BrokerProxySendKeysResponse, error) {
	pi, ok := s.peers.Load(req.PeerId)
	if !ok {
		return nil, errPeeerIDNotFound
	}

	err := pi.(*brokerProxyServiceHelper).SendKeys(req.Keys)
	return &pb.BrokerProxySendKeysResponse{}, err
}

// ReceiveKeys ...
func (s *BrokerProxyService) ReceiveKeys(ctx context.Context, req *pb.BrokerProxyReceiveKeysRequest) (*pb.BrokerProxyReceiveKeysResponse, error) {
	pi, ok := s.peers.Load(req.PeerId)
	if !ok {
		return nil, errPeeerIDNotFound
	}

	keys, err := pi.(*brokerProxyServiceHelper).ReceiveKeys(req.Keys)
	return &pb.BrokerProxyReceiveKeysResponse{Keys: keys}, err
}

// Data ...
func (s *BrokerProxyService) Data(ctx context.Context, r *pb.BrokerProxyDataRequest) (*pb.BrokerProxyDataResponse, error) {
	pi, ok := s.peers.Load(r.PeerId)
	if !ok {
		return nil, errPeeerIDNotFound
	}

	if _, err := pi.(*brokerProxyServiceHelper).w.Write(r.Data); err != nil {
		return nil, err
	}

	return &pb.BrokerProxyDataResponse{}, nil
}

type brokerProxyServiceHelper struct {
	broker Broker
	conn   *bufio.ReadWriter
	w      io.Writer
}

// SendKeys ...
func (h *brokerProxyServiceHelper) SendKeys(keys [][]byte) error {
	return h.broker.SendKeys(h.conn, keys)
}

// ReceiveKeys ...
func (h *brokerProxyServiceHelper) ReceiveKeys(keys [][]byte) ([][]byte, error) {
	return h.broker.ReceiveKeys(h.conn, keys)
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
		return nil, errors.New("response closed unexpectedly")
	}
	o, ok := e.Body.(*pb.BrokerProxyEvent_Open_)
	if !ok {
		cancel()
		return nil, errors.New("unexpected event type")
	}

	p := &brokerProxyClientHelper{
		ctx:    ctx,
		cancel: cancel,
		id:     o.Open.PeerId,
		client: h.client,
		conn:   conn,
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
}

func (p *brokerProxyClientHelper) doEventPump(events chan *pb.BrokerProxyEvent) {
	for {
		select {
		case <-p.ctx.Done():
			return
		case e, ok := <-events:
			if !ok {
				return
			}
			b, ok := e.Body.(*pb.BrokerProxyEvent_Data_)
			if !ok {
				return
			}
			if _, err := p.conn.Write(b.Data.Data); err != nil {
				return
			}
			if err := p.conn.Flush(); err != nil {
				return
			}
		}
	}
}

func (p *brokerProxyClientHelper) doReadPump() {
	b := make([]byte, connMTU(p.conn))
	for {
		n, err := p.conn.Read(b)
		if err != nil {
			return
		}

		req := &pb.BrokerProxyDataRequest{
			PeerId: p.id,
			Data:   b[:n],
		}
		if err := p.client.Data(p.ctx, req, &pb.BrokerProxyDataResponse{}); err != nil {
			return
		}
	}
}

// SendKeys ...
func (p *brokerProxyClientHelper) SendKeys(keys [][]byte) error {
	req := &pb.BrokerProxySendKeysRequest{
		PeerId: p.id,
		Keys:   keys,
	}
	return p.client.SendKeys(p.ctx, req, &pb.BrokerProxySendKeysResponse{})
}

// ReceiveKeys ...
func (p *brokerProxyClientHelper) ReceiveKeys(keys [][]byte) ([][]byte, error) {
	req := &pb.BrokerProxyReceiveKeysRequest{
		PeerId: p.id,
		Keys:   keys,
	}
	res := &pb.BrokerProxyReceiveKeysResponse{}
	if err := p.client.ReceiveKeys(p.ctx, req, res); err != nil {
		return nil, err
	}
	return res.Keys, nil
}

func (p *brokerProxyClientHelper) Close() {
	p.cancel()
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
