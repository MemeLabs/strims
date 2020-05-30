package service

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"log"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"golang.org/x/sync/errgroup"
	"google.golang.org/protobuf/proto"
)

var chatChunkSize = 8

func NewChatThing(ch chan *pb.ChatClientEvent, d vpnData) *ChatThing {
	key := &pb.Key{}
	err := json.Unmarshal([]byte(`{"type":1,"private":"xIbkrrbgy24ps/HizaIsik1X0oAO2CSq9bAFDHa5QtfS4l/CTqSzU7BlqiQa1cOeQR94FZCN0RJuqoYgirV+Mg==","public":"0uJfwk6ks1OwZaokGtXDnkEfeBWQjdESbqqGIIq1fjI="}`), &key)
	if err != nil {
		panic(err)
	}

	return &ChatThing{
		ch:     ch,
		d:      d,
		key:    key,
		events: make(chan *pb.ChatClientEvent, 1),
	}
}

var idThing uint32

// ChatThing ...
type ChatThing struct {
	ch     chan *pb.ChatClientEvent
	d      vpnData
	p      vpn.PeerIndexPublisher
	s      vpn.HashTablePublisher
	key    *pb.Key
	events chan *pb.ChatClientEvent
	ts     *chatServer
	tc     *chatClient
}

// RunServer ...
func (t *ChatThing) RunServer() {
	opt := t.d.controller.ServiceOptions(1)

	port, err := opt.Network.ReservePort()
	if err != nil {
		panic(err)
	}

	s, err := newChatServer(t.key)
	if err != nil {
		panic(err)
	}
	t.ts = s

	err = opt.Network.SetHandler(port, s)
	if err != nil {
		panic(err)
	}

	t.p, err = opt.PeerIndex.Publish(t.key.Public, []byte("chat"), 0)
	if err != nil {
		panic(err)
	}
	log.Println("publishing was ok...")

	// TODO: add ecdh key
	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: t.d.host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	t.s, err = opt.HashTable.Set(t.key, []byte("chat"), b)
	if err != nil {
		panic(err)
	}

	opt.Swarms.OpenSwarm(s.w.Swarm())
}

// RunClient ...
func (t *ChatThing) RunClient() {
	opt := t.d.controller.ServiceOptions(1)

	var addr *pb.NetworkAddress
	var peers []*vpn.PeerIndexHost

	var wg errgroup.Group
	wg.Go(func() (err error) {
		addr, err = t.getHostAddr()
		return
	})
	wg.Go(func() (err error) {
		peers, err = getPeersGetter(opt, t.key, []byte("chat"))()
		return
	})
	if err := wg.Wait(); err != nil {
		panic(err)
	}

	c, err := newChatClient(opt.Network, addr)
	if err != nil {
		panic(err)
	}
	t.tc = c

	jsonDump(addr)
	jsonDump(peers)

	for _, peer := range peers {
		opt.PeerExchange.Connect(peer.HostID)
	}

	swarm, err := encoding.NewSwarm(
		encoding.NewSwarmID(t.key.Public),
		// encoding.NewDefaultSwarmOptions(),
		encoding.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
		},
	)
	if err != nil {
		panic(err)
	}
	opt.Swarms.OpenSwarm(swarm)

	t.p, err = opt.PeerIndex.Publish(t.key.Public, []byte("chat"), 0)
	if err != nil {
		panic(err)
	}

	go func() {
		time.Sleep(2 * time.Second)
		r := swarm.Reader()
		cr, err := chunkstream.NewReaderSize(r, int64(r.Offset()), chatChunkSize)
		if err != nil {
			panic(err)
		}

		log.Println("offset", r.Offset())

		// b := make([]byte, 1024*64)
		b := bytes.NewBuffer(nil)
		for {
			_, err := io.Copy(b, cr)
			if err != nil {
				panic(err)
			}

			// log.Println("read bytes", b.Len(), spew.Sdump(b.Bytes()))

			var msg pb.ChatClientEvent
			err = proto.Unmarshal(b.Bytes(), &msg)
			b.Reset()

			if err != nil {
				log.Println(err)
				continue
			}
			t.events <- &msg

		}
	}()
}

// SendMessage ...
func (t *ChatThing) SendMessage(msg *pb.ChatClientCallRequest_Message) {
	t.tc.Send(msg)
}

// TODO: refactor as utility...
func (t *ChatThing) getHostAddr() (*pb.NetworkAddress, error) {
	opt := t.d.controller.ServiceOptions(1)

	r, err := opt.HashTable.Get(t.key.Public, []byte("chat"))
	if err != nil {
		return nil, err
	}

	addrBytes, ok := latestHashValue(r, time.Second)
	if !ok {
		return nil, errors.New("no addr received")
	}

	addr := &pb.NetworkAddress{}
	if err := proto.Unmarshal(addrBytes, addr); err != nil {
		return nil, err
	}

	return addr, nil
}

func newChatServer(key *pb.Key) (*chatServer, error) {
	w, err := encoding.NewWriter(encoding.SwarmWriterOptions{
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

	s := &chatServer{
		w:      w,
		events: make(chan *pb.ChatClientEvent, 1),
	}

	go s.do(key)

	return s, nil
}

type chatServer struct {
	w      *encoding.SwarmWriter
	events chan *pb.ChatClientEvent
}

func (s *chatServer) do(key *pb.Key) {
	cw, err := chunkstream.NewWriterSize(s.w, chatChunkSize)
	if err != nil {
		panic(err)
	}

	for event := range s.events {
		b, err := proto.Marshal(event)
		if err != nil {
			panic(err)
		}
		// log.Println("wrote bytes", len(b), spew.Sdump(b))

		if _, err := cw.Write(b); err != nil {
			panic(err)
		}
		if err := cw.Flush(); err != nil {
			panic(err)
		}
	}
}

func (s *chatServer) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	var req pb.ChatClientCallRequest
	if err := proto.Unmarshal(msg.Body, &req); err != nil {
		return false, err
	}

	// jsonDump(req)

	switch b := req.Body.(type) {
	case *pb.ChatClientCallRequest_Message_:
		s.events <- &pb.ChatClientEvent{
			Body: &pb.ChatClientEvent_Message_{
				Message: &pb.ChatClientEvent_Message{
					SentTime:   b.Message.Time,
					ServerTime: time.Now().UnixNano() / int64(time.Millisecond),
					Body:       b.Message.Body,
				},
			},
		}
		s.events <- &pb.ChatClientEvent{
			Body: &pb.ChatClientEvent_Padding_{
				Padding: &pb.ChatClientEvent_Padding{
					Body: make([]byte, 1024),
				},
			},
		}
	default:
		log.Printf("some other message type? %T", req.Body)
	}

	return true, nil
}

func newChatClient(network *vpn.Network, addr *pb.NetworkAddress) (*chatClient, error) {
	remoteHostID, err := kademlia.UnmarshalID(addr.HostId)
	if err != nil {
		return nil, err
	}

	port, err := network.ReservePort()
	if err != nil {
		return nil, err
	}

	return &chatClient{
		network:      network,
		remoteHostID: remoteHostID,
		remotePort:   uint16(addr.Port),
		port:         port,
	}, nil
}

type chatClient struct {
	network      *vpn.Network
	remoteHostID kademlia.ID
	remotePort   uint16
	port         uint16
}

func (c *chatClient) Send(msg *pb.ChatClientCallRequest_Message) {
	b, err := proto.Marshal(&pb.ChatClientCallRequest{
		Body: &pb.ChatClientCallRequest_Message_{
			Message: msg,
		},
	})
	if err != nil {
		panic(err)
	}

	err = c.network.Send(c.remoteHostID, c.remotePort, c.port, b)
	if err != nil {
		panic(err)
	}
	// log.Println("sent...?")
}

type byteReader struct {
	b [1]byte
	io.Reader
}

func (r byteReader) ReadByte() (byte, error) {
	_, err := r.Read(r.b[:])
	return r.b[0], err
}

// PeerSearchFunc ...
type PeerSearchFunc func() ([]*vpn.PeerIndexHost, error)

func getPeersGetter(opt NetworkServices, key *pb.Key, salt []byte) PeerSearchFunc {
	// TODO: peer set feature?
	// TDOO: find peers swarm interface function thing...
	return func() ([]*vpn.PeerIndexHost, error) {
		peers := newPeerSet()
		if err := peers.LoadFrom(opt.PeerIndex, key.Public, salt, time.Second); err != nil {
			return nil, err
		}

		return peers.Slice(), nil
	}
}
