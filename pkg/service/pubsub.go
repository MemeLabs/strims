package service

import (
	"bytes"
	"context"
	"errors"
	"io"
	"log"
	"math"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"google.golang.org/protobuf/proto"
)

const syncAddrRetryIvl = 5 * time.Second
const syncAddrRefreshIvl = 10 * time.Minute

// PubSubServerOptions ...
type PubSubServerOptions struct {
	Key *pb.Key
}

// NewPubSubServer ...
func NewPubSubServer(svc *NetworkServices, key *pb.Key, salt []byte) (*PubSubServer, error) {
	w, err := encoding.NewWriter(encoding.SwarmWriterOptions{
		// SwarmOptions: encoding.NewDefaultSwarmOptions(),
		SwarmOptions: encoding.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  128,
		},
		Key: key,
	})
	if err != nil {
		log.Println("error creating writer", err)
		return nil, err
	}

	svc.Swarms.OpenSwarm(w.Swarm())

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	// TODO: add ecdh key
	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: svc.Host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	_, err = svc.HashTable.Set(ctx, key, salt, b)
	if err != nil {
		cancel()
		return nil, err
	}

	if err := svc.PeerIndex.Publish(ctx, key.Public, salt, 0); err != nil {
		cancel()
		return nil, err
	}

	s := &PubSubServer{
		close:    cancel,
		messages: make(chan *pb.PubSubEvent_Message),
		w:        prefixstream.NewWriter(w),
	}

	err = svc.Network.SetHandler(port, s)
	if err != nil {
		cancel()
		return nil, err
	}

	return s, nil
}

// PubSubServer ...
type PubSubServer struct {
	close     context.CancelFunc
	closeOnce sync.Once
	messages  chan *pb.PubSubEvent_Message
	w         *prefixstream.Writer
}

// Close ...
func (s *PubSubServer) Close() {
	s.closeOnce.Do(func() {
		s.close()
		close(s.messages)
	})
}

// Messages ...
func (s *PubSubServer) Messages() <-chan *pb.PubSubEvent_Message {
	return s.messages
}

// Send ...
func (s *PubSubServer) Send(key string, body []byte) error {
	n, err := s.send(&pb.PubSubEvent{
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
				Body: make([]byte, 128-(n%128)),
			},
		},
	})
	if err != nil {
		return err
	}

	return nil
}

func (s *PubSubServer) send(e *pb.PubSubEvent) (int, error) {
	b, err := proto.Marshal(e)
	if err != nil {
		return 0, err
	}

	if _, err := s.w.Write(b); err != nil {
		return 0, err
	}
	return len(b), nil
}

// HandleMessage ...
func (s *PubSubServer) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	var req pb.PubSubEvent
	if err := proto.Unmarshal(msg.Body, &req); err != nil {
		return false, err
	}

	switch b := req.Body.(type) {
	case *pb.PubSubEvent_Message_:
		s.messages <- b.Message
	default:
		log.Printf("some other message type? %T", req.Body)
	}

	return true, nil
}

// NewPubSubClient ...
func NewPubSubClient(svc *NetworkServices, key, salt []byte) (*PubSubClient, error) {
	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	swarm, err := encoding.NewSwarm(
		encoding.NewSwarmID(key),
		// encoding.NewDefaultSwarmOptions(),
		encoding.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  128,
		},
	)
	if err != nil {
		return nil, err
	}
	svc.Swarms.OpenSwarm(swarm)

	messages := make(chan *pb.PubSubEvent_Message)
	go readPubSubEvents(swarm, messages)

	ctx, cancel := context.WithCancel(context.Background())

	err = svc.PeerIndex.Publish(ctx, key, salt, 0)
	if err != nil {
		svc.Swarms.CloseSwarm(swarm.ID)
		cancel()
		return nil, err
	}

	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key, salt))

	c := &PubSubClient{
		ctx:       ctx,
		close:     cancel,
		network:   svc.Network,
		addrReady: make(chan struct{}),
		port:      port,
		messages:  messages,
	}

	go c.syncAddr(svc, key, salt)

	return c, nil
}

// PubSubClient ...
type PubSubClient struct {
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	network   *vpn.Network
	addrReady chan struct{}
	addr      atomic.Value
	port      uint16
	messages  chan *pb.PubSubEvent_Message
}

func (c *PubSubClient) syncAddr(svc *NetworkServices, key, salt []byte) {
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
func (c *PubSubClient) Close() {
	c.closeOnce.Do(func() {
		c.close()
		close(c.messages)
	})
}

// Messages ...
func (c *PubSubClient) Messages() <-chan *pb.PubSubEvent_Message {
	return c.messages
}

// Send ...
func (c *PubSubClient) Send(key string, body []byte) error {
	select {
	case <-c.addrReady:
	case <-c.ctx.Done():
	}
	if c.ctx.Err() != nil {
		return c.ctx.Err()
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
	return c.network.Send(addr.HostID, addr.Port, c.port, b)
}

func readPubSubEvents(swarm *encoding.Swarm, messages chan *pb.PubSubEvent_Message) {
	r := prefixstream.NewReader(swarm.Reader())
	b := bytes.NewBuffer(nil)
	for {
		b.Reset()
		if _, err := io.Copy(b, r); err != nil {
			return
		}

		var msg pb.PubSubEvent
		if proto.Unmarshal(b.Bytes(), &msg) != nil {
			continue
		}

		switch b := msg.Body.(type) {
		case *pb.PubSubEvent_Close_:
			// TODO: this has to call c.Close()
			close(messages)
			return
		case *pb.PubSubEvent_Message_:
			messages <- b.Message
		}
	}
}

func newSwarmPeerManager(ctx context.Context, svc *NetworkServices, sf PeerSearchFunc) *swarmPeerManager {
	m := &swarmPeerManager{
		ctx: ctx,
		svc: svc,
		sf:  sf,
	}

	m.poller = vpn.NewPoller(ctx, 5*time.Minute, m.update, nil)

	return m
}

type swarmPeerManager struct {
	ctx    context.Context
	svc    *NetworkServices
	sf     PeerSearchFunc
	poller *vpn.Poller
}

func (m *swarmPeerManager) update(_ time.Time) {
	peers, err := m.sf()
	if err != nil {
		return
	}

	for _, peer := range peers {
		m.svc.PeerExchange.Connect(peer.HostID)
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
		ctx, _ := context.WithTimeout(ctx, getPeersGetterTimeout)
		peers := newPeerSet()
		if err := peers.LoadFrom(ctx, svc.PeerIndex, key, salt); err != nil {
			return nil, err
		}

		return peers.Slice(), nil
	}
}

const latestHashValuesTimeout = 5 * time.Second

func latestHashValue(ctx context.Context, svc *NetworkServices, key, salt []byte) ([]byte, error) {
	ctx, _ = context.WithTimeout(ctx, latestHashValuesTimeout)
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
