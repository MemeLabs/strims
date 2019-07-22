package encoding

import (
	"context"
	"errors"
	"log"
	"math"
	"math/rand"
	"net"
	"sync"
	"sync/atomic"
	"time"
)

type HostOptions struct {
	Context    context.Context
	Transports []Transport
}

func NewDefaultHostOptions() *HostOptions {
	return &HostOptions{
		Context: context.Background(),
		Transports: []Transport{
			&UDPTransport{
				Address: ":7881",
			},
		},
	}
}

func NewHost(o *HostOptions) (c *Host) {
	c = &Host{
		ctx:      o.Context,
		swarms:   map[string]*Swarm{},
		peers:    map[string]*Peer{},
		channels: NewChannelsMap(),
	}

	c.servers = make([]*MemeServer, len(o.Transports))
	for i, t := range o.Transports {
		c.servers[i] = &MemeServer{
			t:       t,
			Handler: c.handleMemeRequest,
		}
	}

	return c
}

type Host struct {
	ctx     context.Context
	servers []*MemeServer
	nextUID int64
	swarms  map[string]*Swarm
	// TODO: this should be a kad routing table
	peersLock sync.Mutex
	peers     map[string]*Peer
	channels  *ChannelsMap
}

func (h *Host) NextUID() int64 {
	return atomic.AddInt64(&h.nextUID, 1)
}

func (h *Host) createChannelID() uint32 {
	for {
		channelID := uint32(rand.Int63n(math.MaxUint32))
		if _, ok := h.channels.Get(channelID); !ok {
			return channelID
		}
	}
}

func (h *Host) Run() (err error) {
	defer func() {
		if err != nil {
			h.Shutdown()
		}
	}()

	ctx, cancel := context.WithCancel(h.ctx)

	wg := &sync.WaitGroup{}
	wg.Add(len(h.servers))

	go h.timeoutPeerChannels(ctx)

	for _, s := range h.servers {
		go func(s *MemeServer) {
			if err := s.Listen(ctx); err != nil {
				log.Println("server error", err)
			}
			cancel()
			wg.Done()
		}(s)
	}

	wg.Wait()
	cancel()

	// TODO: propagate errors

	return
}

func (h *Host) Shutdown() (err error) {
	log.Println("shutting down")

	for _, s := range h.servers {
		if err = s.Shutdown(); err != nil {
			log.Println("error closing transport:", err)
		}
	}
	return
}

func (h *Host) timeoutPeerChannels(ctx context.Context) {
	checkIvl := 5 * time.Second
	timeout := 5 * time.Second

	t := time.NewTicker(checkIvl)
	for {
		select {
		case <-t.C:
		case <-ctx.Done():
			return
		}

		removed := []*Peer{}

		deadline := time.Now().Add(-timeout).Unix()
		h.peersLock.Lock()
		for k, p := range h.peers {
			if p.LastActive() < deadline {
				delete(h.peers, k)
				removed = append(removed, p)
			}
		}
		h.peersLock.Unlock()

		for _, p := range removed {
			// TODO: encapsulation? lul
			for id, c := range p.channels.channels {
				c.p.channels.Delete(id)
			}
		}
	}
}

func (h *Host) handleMemeRequest(w MemeWriter, r *MemeRequest) {
	if len(r.Messages) == 0 {
		return
	}

	channel, ok := h.channels.Get(r.ChannelID)
	if !ok {
		var err error
		channel, err = h.createChannel(r)
		if err != nil {
			return
		}
	}
	channel.p.UpdateLastActive()
	channel.HandleMemeRequest(w, r)

	// TODO: check dgram channelID matches req transport addr?
	// channel.s.handleMemeRequest(w, r)

	// spew.Dump(map[string]interface{}{"w": w, "r": r})
}

func (h *Host) createChannel(r *MemeRequest) (*PeerChannel, error) {
	// spew.Dump(w, r)
	log.Println("began handshake...")

	if r.ChannelID != 0 {
		return nil, errors.New("handshake: non-zero channel id")
	}

	handshake, ok := r.Messages[0].(*Handshake)
	if !ok {
		return nil, errors.New("handshake: first message is not handshake")
	}

	swarmIDOptionIf, ok := handshake.Options.Find(SwarmIdentifierOption)
	if !ok {
		return nil, errors.New("handshake: handshake has no swarm id")
	}
	swarmID := &SwarmID{
		PublicKey: *swarmIDOptionIf.(*SwarmIdentifierProtocolOption),
	}
	swarm, ok := h.swarms[swarmID.String()]
	if !ok {
		return nil, errors.New("handshake: unknown swarm")
	}

	// TODO: one channel per peer per swarm

	h.peersLock.Lock()
	peer, ok := h.peers[r.Conn.String()]
	if !ok {
		peer = NewPeer(h.NextUID())
		h.peers[r.Conn.String()] = peer
	}
	h.peersLock.Unlock()

	// TODO: limit channels per peer?

	pc := &PeerChannel{
		ID:       h.createChannelID(),
		RemoteID: handshake.ChannelID,
		s:        swarm,
		p:        peer,
		// t:  Transport
		// c: w.c,
	}
	h.channels.Insert(pc.ID, pc)
	peer.channels.Insert(pc.ID, pc)

	return pc, nil
}

func (h *Host) TestSend(addr string) (err error) {
	log.Println(addr)

	s := &Swarm{
		UID: h.NextUID(),
		ID: &SwarmID{
			PublicKey: make([]byte, 64),
		},
		channels: NewChannelsMap(),
	}

	p := NewPeer(h.NextUID())

	t := h.servers[0].t

	c := &UDPConn{
		t: t.(*UDPTransport),
	}

	pc := &PeerChannel{
		ID: h.createChannelID(),
		s:  s,
		p:  p,
		// t:  t,
		// c:  c,
	}

	c.a, err = net.ResolveUDPAddr("udp", addr)
	if err != nil {
		return
	}

	// this should be the remote id
	h.channels.Insert(pc.ID, pc)
	h.swarms[s.ID.String()] = s
	h.peers[c.String()] = p

	for {
		time.Sleep(time.Second)

		swarmID := SwarmIdentifierProtocolOption(s.ID.Binary())

		d := &Datagram{
			ChannelID: 0,
			Messages: []Message{
				&Handshake{
					ChannelID: pc.ID,
					Options: []ProtocolOption{
						&VersionProtocolOption{Value: 1},
						&MinimumVersionProtocolOption{Value: 1},
						&swarmID,
					},
				},
				&Request{NewBin32ChunkAddress(16)},
				NewData(Bin(16), make(Buffer, 128)),
			},
		}
		buf := make([]byte, 1500)
		n := d.Marshal(buf)
		// spew.Dump(buf[:n])
		err = c.Write(buf[:n])
		if err != nil {
			return
		}
	}
}

func (h *Host) TestReceive() (err error) {
	s := NewSwarm(
		h.NextUID(),
		&SwarmID{
			PublicKey: make([]byte, 64),
		},
	)

	h.swarms[s.ID.String()] = s

	select {}
}
