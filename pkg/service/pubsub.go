package service

import (
	"bytes"
	"context"
	"errors"
	"io"
	"log"
	"math"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"google.golang.org/protobuf/proto"
)

var pubSubChunkSize = 8

// PubSubServerOptions ...
type PubSubServerOptions struct {
	Key *pb.Key
}

// NewPubSubServer ...
func NewPubSubServer(ctx context.Context, svc *NetworkServices, key *pb.Key, salt []byte) (*PubSubServer, error) {
	sw, err := encoding.NewWriter(encoding.SwarmWriterOptions{
		// SwarmOptions: encoding.NewDefaultSwarmOptions(),
		SwarmOptions: encoding.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
		},
		Key: key,
	})
	if err != nil {
		log.Println("error creating writer", err)
		return nil, err
	}

	cw, err := chunkstream.NewWriterSize(sw, pubSubChunkSize)
	if err != nil {
		return nil, err
	}

	svc.Swarms.OpenSwarm(sw.Swarm())

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(ctx)

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
		close:     cancel,
		publishes: make(chan *pb.PubSubEvent_Publish),
		cw:        cw,
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
	publishes chan *pb.PubSubEvent_Publish
	cw        *chunkstream.Writer
}

// Close ...
func (s *PubSubServer) Close() {
	s.close()
}

// Publishes ...
func (s *PubSubServer) Publishes() <-chan *pb.PubSubEvent_Publish {
	return s.publishes
}

// Send ...
func (s *PubSubServer) Send(key string, body []byte) error {
	n, err := s.send(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Message_{
			Message: &pb.PubSubEvent_Message{
				ServerTime: time.Now().Unix(),
				Key:        key,
				Body:       body,
			},
		},
	})
	if err != nil {
		return err
	}

	_, err = s.send(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Padding_{
			Padding: &pb.PubSubEvent_Padding{
				Body: make([]byte, n%1024),
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
	if _, err := s.cw.Write(b); err != nil {
		return 0, err
	}
	if err := s.cw.Flush(); err != nil {
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
	case *pb.PubSubEvent_Publish_:
		s.publishes <- b.Publish
	default:
		log.Printf("some other message type? %T", req.Body)
	}

	return true, nil
}

// NewPubSubClient ...
func NewPubSubClient(ctx context.Context, svc *NetworkServices, key, salt []byte) (*PubSubClient, error) {
	addr, err := getHostAddr(ctx, svc, key, salt)
	if err != nil {
		return nil, err
	}

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	swarm, err := encoding.NewSwarm(
		encoding.NewSwarmID(key),
		// encoding.NewDefaultSwarmOptions(),
		encoding.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
		},
	)
	if err != nil {
		return nil, err
	}
	svc.Swarms.OpenSwarm(swarm)

	messages := make(chan *pb.PubSubEvent_Message)
	go readPubSubEvents(swarm, messages)

	ctx, cancel := context.WithCancel(ctx)

	err = svc.PeerIndex.Publish(ctx, key, salt, 0)
	if err != nil {
		svc.Swarms.CloseSwarm(swarm.ID)
		cancel()
		return nil, err
	}

	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key, salt))

	return &PubSubClient{
		close:      cancel,
		network:    svc.Network,
		remoteAddr: addr,
		port:       port,
		messages:   messages,
	}, nil
}

// PubSubClient ...
type PubSubClient struct {
	close      context.CancelFunc
	network    *vpn.Network
	remoteAddr *hostAddr
	port       uint16
	messages   chan *pb.PubSubEvent_Message
}

// Close ...
func (c *PubSubClient) Close() {
	c.close()
}

// Messages ...
func (c *PubSubClient) Messages() <-chan *pb.PubSubEvent_Message {
	return c.messages
}

// Publish ...
func (c *PubSubClient) Publish(key string, body []byte) error {
	b, err := proto.Marshal(&pb.PubSubEvent{
		Body: &pb.PubSubEvent_Publish_{
			Publish: &pb.PubSubEvent_Publish{
				Time: time.Now().UnixNano() / int64(time.Millisecond),
				Key:  key,
				Body: body,
			},
		},
	})
	if err != nil {
		return err
	}

	return c.network.Send(c.remoteAddr.HostID, c.remoteAddr.Port, c.port, b)
}

func readPubSubEvents(swarm *encoding.Swarm, messages chan *pb.PubSubEvent_Message) {
	r := swarm.Reader()
	cr, err := chunkstream.NewReaderSize(r, int64(r.Offset()), pubSubChunkSize)
	if err != nil {
		panic(err)
	}

	log.Println("offset", r.Offset())

	b := bytes.NewBuffer(nil)
	for {
		b.Reset()
		_, err := io.Copy(b, cr)
		if err != nil {
			panic(err)
		}

		var msg pb.PubSubEvent
		if proto.Unmarshal(b.Bytes(), &msg) != nil {
			continue
		}

		switch b := msg.Body.(type) {
		case *pb.PubSubEvent_Close_:
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
