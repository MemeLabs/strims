package pubsub

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"log"
	"math"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const pubSubChunkSize = 128
const syncAddrRetryIvl = 5 * time.Second
const syncAddrRefreshIvl = 10 * time.Minute

// ServerOptions ...
type ServerOptions struct {
	Key *pb.Key
}

// NewServer ...
func NewServer(svc *NetworkServices, key *pb.Key, salt []byte) (*Server, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		// SwarmOptions: ppspp.NewDefaultSwarmOptions(),
		SwarmOptions: ppspp.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  pubSubChunkSize,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod: integrity.ProtectionMethodSignAll,
			},
		},
		Key: key,
	})
	if err != nil {
		return nil, err
	}

	svc.Swarms.OpenSwarm(w.Swarm())

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key.Public, salt))

	// TODO: add ecdh key
	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: svc.Host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	if err != nil {
		cancel()
		return nil, err
	}
	_, err = svc.HashTable.Set(ctx, key, salt, b)
	if err != nil {
		cancel()
		return nil, err
	}

	if err := svc.PeerIndex.Publish(ctx, key.Public, salt, 0); err != nil {
		cancel()
		return nil, err
	}

	s := &Server{
		logger:   svc.Host.Logger(),
		close:    cancel,
		messages: make(chan *pb.PubSubEvent_Message),
		swarm:    w.Swarm(),
		svc:      svc,
		w:        prefixstream.NewWriter(w),
	}

	err = svc.Network.SetHandler(port, s)
	if err != nil {
		cancel()
		return nil, err
	}

	return s, nil
}

// Server ...
type Server struct {
	logger    *zap.Logger
	close     context.CancelFunc
	closeOnce sync.Once
	messages  chan *pb.PubSubEvent_Message
	swarm     *ppspp.Swarm
	svc       *NetworkServices
	w         *prefixstream.Writer
}

// Close ...
func (s *Server) Close() {
	s.closeOnce.Do(func() {
		s.close()
		close(s.messages)
		s.svc.Swarms.CloseSwarm(s.swarm.ID())
	})
}

// Messages ...
func (s *Server) Messages() <-chan *pb.PubSubEvent_Message {
	return s.messages
}

// Send ...
func (s *Server) Send(ctx context.Context, key string, body []byte) error {
	_, err := s.send(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Message_{
			Message: &pb.PubSubEvent_Message{
				Time: time.Now().Unix(),
				Key:  key,
				Body: body,
			},
		},
	})
	if err != nil {
		return err
	}

	_, err = s.send(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Padding_{
			Padding: &pb.PubSubEvent_Padding{
				Body: make([]byte, pubSubChunkSize),
			},
		},
	})
	if err != nil {
		return err
	}

	return nil
}

func (s *Server) send(e *pb.PubSubEvent) (int, error) {
	b := pool.Get(uint16(proto.Size(e)))
	defer pool.Put(b)

	var err error
	*b, err = proto.MarshalOptions{}.MarshalAppend((*b)[:0], e)
	if err != nil {
		return 0, err
	}

	if _, err := s.w.Write(*b); err != nil {
		return 0, err
	}
	return len(*b), nil
}

// HandleMessage ...
func (s *Server) HandleMessage(msg *vpn.Message) error {
	var req pb.PubSubEvent
	if err := proto.Unmarshal(msg.Body, &req); err != nil {
		return err
	}

	switch b := req.Body.(type) {
	case *pb.PubSubEvent_Message_:
		s.messages <- b.Message
	default:
		log.Printf("some other message type? %T", req.Body)
	}

	return nil
}

// NewClient ...
func NewClient(svc *NetworkServices, key, salt []byte) (*Client, error) {
	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	swarm, err := ppspp.NewSwarm(
		ppspp.NewSwarmID(key),
		// ppspp.NewDefaultSwarmOptions(),
		ppspp.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  pubSubChunkSize,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod: integrity.ProtectionMethodSignAll,
			},
		},
	)
	if err != nil {
		return nil, fmt.Errorf("starting pubsub swarm failed: %w", err)
	}
	svc.Swarms.OpenSwarm(swarm)

	ctx, cancel := context.WithCancel(context.Background())

	err = svc.PeerIndex.Publish(ctx, key, salt, 0)
	if err != nil {
		svc.Swarms.CloseSwarm(swarm.ID())
		cancel()
		return nil, fmt.Errorf("publishing swarm to peer index failed: %w", err)
	}

	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key, salt))

	c := &Client{
		logger:    svc.Host.Logger(),
		ctx:       ctx,
		close:     cancel,
		svc:       svc,
		swarm:     swarm,
		addrReady: make(chan struct{}),
		port:      port,
		messages:  make(chan *pb.PubSubEvent_Message),
	}

	go c.syncAddr(svc, key, salt)
	go func() {
		if err := c.readEvents(); err != nil {
			c.logger.Debug("pubsub read error", zap.Error(err))
		}
	}()

	return c, nil
}

// Client ...
type Client struct {
	logger    *zap.Logger
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	svc       *NetworkServices
	swarm     *ppspp.Swarm
	addrReady chan struct{}
	addr      atomic.Value
	port      uint16
	messages  chan *pb.PubSubEvent_Message
}

func (c *Client) syncAddr(svc *NetworkServices, key, salt []byte) {
	var nextTick time.Duration
	var closeOnce sync.Once
	for {
		select {
		case <-time.After(nextTick):
			addr, err := getHostAddr(c.ctx, svc, key, salt)
			if err != nil {
				nextTick = syncAddrRetryIvl
				continue
			}

			c.addr.Store(addr)
			closeOnce.Do(func() { close(c.addrReady) })

			nextTick = syncAddrRefreshIvl
		case <-c.ctx.Done():
			return
		}
	}
}

// Close ...
func (c *Client) Close() {
	c.closeOnce.Do(func() {
		c.close()
		close(c.messages)
		c.svc.Swarms.CloseSwarm(c.swarm.ID())
	})
}

// Messages ...
func (c *Client) Messages() <-chan *pb.PubSubEvent_Message {
	return c.messages
}

// Send ...
func (c *Client) Send(ctx context.Context, key string, body []byte) error {
	select {
	case <-c.addrReady:
	case <-c.ctx.Done():
	case <-ctx.Done():
	}
	if c.ctx.Err() != nil {
		return c.ctx.Err()
	}
	if ctx.Err() != nil {
		return ctx.Err()
	}

	b, err := proto.Marshal(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Message_{
			Message: &pb.PubSubEvent_Message{
				Time: time.Now().UnixNano(),
				Key:  key,
				Body: body,
			},
		},
	})
	if err != nil {
		return err
	}

	addr := c.addr.Load().(*hostAddr)
	return c.svc.Network.Send(addr.HostID, addr.Port, c.port, b)
}

func (c *Client) readEvents() error {
	defer c.Close()

	r := prefixstream.NewReader(c.swarm.Reader())
	b := bytes.NewBuffer(nil)
	for {
		b.Reset()
		if _, err := io.Copy(b, r); err != nil {
			return err
		}

		var msg pb.PubSubEvent
		if err := proto.Unmarshal(b.Bytes(), &msg); err != nil {
			continue
		}

		switch b := msg.Body.(type) {
		case *pb.PubSubEvent_Close_:
			return nil
		case *pb.PubSubEvent_Message_:
			c.messages <- b.Message
		}
	}
}

func newSwarmPeerManager(ctx context.Context, svc *NetworkServices, sf PeerSearchFunc) *swarmPeerManager {
	m := &swarmPeerManager{
		logger: svc.Host.Logger(),
		ctx:    ctx,
		svc:    svc,
		sf:     sf,
	}

	m.ticker = vpn.TickerFunc(ctx, 5*time.Minute, m.update)

	return m
}

type swarmPeerManager struct {
	logger *zap.Logger
	ctx    context.Context
	svc    *NetworkServices
	sf     PeerSearchFunc
	ticker *vpn.Ticker
}

func (m *swarmPeerManager) update(_ time.Time) {
	peers, err := m.sf()
	if err != nil {
		return
	}

	for _, peer := range peers {
		if err := m.svc.PeerExchange.Connect(peer.HostID); err != nil {
			continue
		}
	}
}

type hostAddr struct {
	HostID kademlia.ID
	Port   uint16
}

func getHostAddr(ctx context.Context, svc *NetworkServices, key, salt []byte) (*hostAddr, error) {
	addrBytes, err := latestHashValue(ctx, svc, key, salt)
	if err != nil {
		return nil, err
	}

	addr := &pb.NetworkAddress{}
	if err := proto.Unmarshal(addrBytes, addr); err != nil {
		return nil, err
	}

	svc.Host.Logger().Debug(
		"found address for service",
		logutil.ByteHex("key", key),
		logutil.ByteHex("salt", salt),
		logutil.ByteHex("hostID", addr.HostId),
		zap.Uint32("port", addr.Port),
	)
	hostID, err := kademlia.UnmarshalID(addr.HostId)
	if err != nil {
		return nil, err
	}

	if addr.Port > math.MaxUint16 {
		return nil, errors.New("port out of range")
	}

	return &hostAddr{hostID, uint16(addr.Port)}, nil
}

// PeerSearchFunc ...
type PeerSearchFunc func() ([]*vpn.PeerIndexHost, error)

const getPeersGetterTimeout = 5 * time.Second

func getPeersGetter(ctx context.Context, svc *NetworkServices, key, salt []byte) PeerSearchFunc {
	// TODO: peer set feature?
	// TDOO: find peers swarm interface function thing...
	return func() ([]*vpn.PeerIndexHost, error) {
		ctx, cancel := context.WithTimeout(ctx, getPeersGetterTimeout)
		defer cancel()

		peers := newPeerSet()
		if err := peers.LoadFrom(ctx, svc.PeerIndex, key, salt); err != nil {
			return nil, err
		}

		return peers.Slice(), nil
	}
}

const latestHashValuesTimeout = 5 * time.Second

func latestHashValue(ctx context.Context, svc *NetworkServices, key, salt []byte) ([]byte, error) {
	ctx, cancel := context.WithTimeout(ctx, latestHashValuesTimeout)
	defer cancel()

	values, err := svc.HashTable.Get(ctx, key, salt)
	if err != nil {
		return nil, err
	}

	var timestamp time.Time
	var value []byte
	for v := range values {
		if v.Timestamp.After(timestamp) {
			timestamp = v.Timestamp
			value = v.Value
		}
	}

	if value == nil {
		return nil, errors.New("no value received")
	}
	return value, nil
}
