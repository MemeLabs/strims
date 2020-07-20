// +build js

package vpn

import (
	"bufio"
	"context"
	"errors"
	"io"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
)

var errPeeerIDNotFound = errors.New("peer id not found")

// NewBrokerService ...
func NewBrokerService(logger *zap.Logger) *BrokerService {
	return &BrokerService{
		logger: logger,
		networkBroker: &networkBroker{
			logger: logger,
		},
	}
}

// BrokerService ...
type BrokerService struct {
	logger        *zap.Logger
	peers         sync.Map
	nextPeerID    uint64
	networkBroker *networkBroker
}

// BrokerPeer ...
func (s *BrokerService) BrokerPeer(ctx context.Context, r *pb.BrokerPeerRequest) (chan *pb.BrokerPeerEvent, error) {
	ch := make(chan *pb.BrokerPeerEvent, 1)

	r0, w0 := io.Pipe()
	r1, w1 := io.Pipe()
	c := bufio.NewReadWriter(bufio.NewReader(r0), bufio.NewWriterSize(w1, int(r.ConnMtu)))

	broker, err := s.networkBroker.BrokerPeer(c)
	if err != nil {
		return nil, err
	}

	p := &brokerServicePeer{
		p: broker,
		w: w0,
	}

	pid := atomic.AddUint64(&s.nextPeerID, 1)
	s.peers.Store(pid, p)

	ch <- &pb.BrokerPeerEvent{
		Body: &pb.BrokerPeerEvent_Open_{
			Open: &pb.BrokerPeerEvent_Open{
				PeerId: pid,
			},
		},
	}

	cancel := onceFunc(func() {
		p.p.Close()
		s.peers.Delete(pid)
		close(ch)
	})

	go func() {
		defer cancel()

		for {
			select {
			case keys := <-broker.Keys():
				ch <- &pb.BrokerPeerEvent{
					Body: &pb.BrokerPeerEvent_Keys_{
						Keys: &pb.BrokerPeerEvent_Keys{
							Keys: keys,
						},
					},
				}
			case <-broker.InitRequired():
				ch <- &pb.BrokerPeerEvent{
					Body: &pb.BrokerPeerEvent_InitRequired_{
						InitRequired: &pb.BrokerPeerEvent_InitRequired{},
					},
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	go func() {
		defer cancel()

		b := make([]byte, r.ConnMtu)
		for {
			n, err := r1.Read(b)
			if err != nil {
				return
			}

			ch <- &pb.BrokerPeerEvent{
				Body: &pb.BrokerPeerEvent_Data_{
					Data: &pb.BrokerPeerEvent_Data{
						Data: b[:n],
					},
				},
			}
		}
	}()

	return ch, nil
}

// Init ...
func (s *BrokerService) Init(ctx context.Context, r *pb.BrokerPeerInitRequest) error {
	pi, ok := s.peers.Load(r.PeerId)
	if !ok {
		return errPeeerIDNotFound
	}

	return pi.(*brokerServicePeer).p.Init(uint16(r.Discriminator), r.Keys)
}

// Data ...
func (s *BrokerService) Data(ctx context.Context, r *pb.BrokerPeerDataRequest) error {
	pi, ok := s.peers.Load(r.PeerId)
	if !ok {
		return errPeeerIDNotFound
	}

	_, err := pi.(*brokerServicePeer).w.Write(r.Data)
	return err
}

type brokerServicePeer struct {
	p NetworkBrokerPeer
	w io.WriteCloser
}

// NewBrokerClient ....
func NewBrokerClient(logger *zap.Logger, bus *wasmio.Bus) NetworkBroker {
	client := rpc.NewClient(logger, bus, bus)

	return &BrokerClient{
		logger: logger,
		client: client,
	}
}

// BrokerClient ...
type BrokerClient struct {
	logger *zap.Logger
	client *rpc.Client
}

// BrokerPeer ...
func (h *BrokerClient) BrokerPeer(conn ReadWriteFlusher) (NetworkBrokerPeer, error) {
	ctx, cancel := context.WithCancel(context.Background())

	req := &pb.BrokerPeerRequest{ConnMtu: int32(connMTU(conn))}
	events := make(chan *pb.BrokerPeerEvent, 1)
	if err := h.client.CallStreaming(ctx, "brokerPeer", req, events); err != nil {
		cancel()
		return nil, err
	}

	e, ok := <-events
	if !ok {
		cancel()
		return nil, errors.New("response closed unexpectedly")
	}
	o, ok := e.Body.(*pb.BrokerPeerEvent_Open_)
	if !ok {
		cancel()
		return nil, errors.New("unexpected event type")
	}
	return newBrokerClientPeer(o.Open.PeerId, h.client, conn, events, cancel), nil
}

func newBrokerClientPeer(
	id uint64,
	client *rpc.Client,
	conn ReadWriteFlusher,
	events chan *pb.BrokerPeerEvent,
	cancel func(),
) *brokerClientPeer {
	p := &brokerClientPeer{
		id:           id,
		client:       client,
		conn:         conn,
		cancel:       cancel,
		initRequired: make(chan struct{}),
		keys:         make(chan [][]byte),
	}

	go p.doEventPump(events)
	go p.doReadPump()

	return p
}

type brokerClientPeer struct {
	id           uint64
	client       *rpc.Client
	conn         ReadWriteFlusher
	cancel       func()
	initRequired chan struct{}
	keys         chan [][]byte
}

func (p *brokerClientPeer) doEventPump(events chan *pb.BrokerPeerEvent) {
	defer p.cancel()

	for e := range events {
		switch b := e.Body.(type) {
		case *pb.BrokerPeerEvent_InitRequired_:
			p.initRequired <- struct{}{}
		case *pb.BrokerPeerEvent_Data_:
			if _, err := p.conn.Write(b.Data.Data); err != nil {
				return
			}
			if err := p.conn.Flush(); err != nil {
				return
			}
		case *pb.BrokerPeerEvent_Keys_:
			p.keys <- b.Keys.Keys
		default:
			return
		}
	}
}

func (p *brokerClientPeer) doReadPump() {
	defer p.cancel()

	b := make([]byte, connMTU(p.conn))
	for {
		n, err := p.conn.Read(b)
		if err != nil {
			return
		}

		req := &pb.BrokerPeerDataRequest{
			PeerId: p.id,
			Data:   b[:n],
		}
		if err := p.client.Call(context.Background(), "data", req); err != nil {
			return
		}
	}
}

// Init ...
func (p *brokerClientPeer) Init(discriminator uint16, keys [][]byte) error {
	req := &pb.BrokerPeerInitRequest{
		PeerId:        p.id,
		Discriminator: uint32(discriminator),
		Keys:          keys,
	}
	return p.client.Call(context.Background(), "init", req)
}

func (p *brokerClientPeer) InitRequired() <-chan struct{} {
	return p.initRequired
}

func (p *brokerClientPeer) Keys() <-chan [][]byte {
	return p.keys
}

func (p *brokerClientPeer) Close() {
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

func onceFunc(f func()) func() {
	var once sync.Once
	return func() { once.Do(f) }
}
