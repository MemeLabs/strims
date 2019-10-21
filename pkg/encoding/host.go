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

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/nareix/joy4/format"
)

func init() {
	format.RegisterAll()
}

// HostOptions ...
type HostOptions struct {
	Context    context.Context
	Transports []Transport
}

// NewDefaultHostOptions ...
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

// NewHost ...
func NewHost(o *HostOptions) (c *Host) {
	c = &Host{
		ctx:       o.Context,
		scheduler: NewScheduler(o.Context),
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

// Host ...
type Host struct {
	sync.Mutex

	ctx       context.Context
	prevUID   int64
	servers   []*MemeServer
	swarms    sync.Map
	peers     sync.Map
	channels  sync.Map
	scheduler *Scheduler
}

// nextUID ...
func (h *Host) nextUID() int64 {
	return atomic.AddInt64(&h.prevUID, 1)
}

// Close ...
func (h *Host) Close() (err error) {
	// h.scheduler.Close()
	return
}

// HostSwarm ...
func (h *Host) HostSwarm(s *Swarm) {
	h.swarms.Store(s.ID.String(), s)
}

// JoinSwarm ...
func (h *Host) JoinSwarm(id *SwarmID, addr string) (err error) {
	s := NewDefaultSwarm(id)
	h.swarms.Store(id.String(), s)

	c := UDPConn{
		t: h.servers[0].t.(*UDPTransport),
	}
	c.a, err = net.ResolveUDPAddr("udp", addr)
	if err != nil {
		return
	}

	cid := uint32(rand.Int63n(math.MaxUint32))
	p := h.peer(c)
	ch := newChannel(channelOptions{
		ID:    cid,
		Swarm: s,
		Peer:  p,
		Conn:  c,
	})
	h.channels.Store(cid, ch)
	p.channels.Store(cid, ch)
	s.channels.Store(cid, ch)

	w := NewMemeWriter(c)
	ch.OfferHandshake(w)
	w.Flush()

	go func() {
		s.chunks.debug = true

		cr := s.chunks.Reader()
		log.Println("offset", int64(cr.Offset()))
		r, err := chunkstream.NewReader(cr, int64(cr.Offset()))
		if err != nil {
			log.Panic(err)
		}
		b := make([]byte, 5*1024*1024)
		bn := 0
		rb := b
		for {
			n, err := r.Read(rb)
			if err == chunkstream.EOR {
				debug.Green("got chunk", bn)
				rb = b
			} else if err != nil {
				log.Println("read failed with error", err)
				break
			}
			bn += n

			if h.ctx.Err() == context.Canceled {
				break
			}
		}
	}()

	// TODO: probably return a reader?
	return
}

// RemoveSwarm ...
func (h *Host) RemoveSwarm(id *SwarmID) {
	h.swarms.Delete(id.String())
}

// Run ...
func (h *Host) Run() (err error) {
	defer func() {
		if err != nil {
			h.Shutdown()
		}
	}()

	ctx, cancel := context.WithCancel(h.ctx)

	wg := &sync.WaitGroup{}

	// TODO: uncomment this...
	// go h.timeoutPeerChannels(ctx)

	wg.Add(len(h.servers))
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

// Shutdown ...
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
	timeout := 30 * time.Second

	t := time.NewTicker(checkIvl)
	for {
		select {
		case <-t.C:
		case <-ctx.Done():
			return
		}

		deadline := time.Now().Add(-timeout)
		removed := map[*Peer]bool{}
		h.peers.Range(func(id interface{}, pi interface{}) bool {
			if deadline.After(pi.(*Peer).LastActive()) {
				removed[pi.(*Peer)] = true
				h.peers.Delete(id)
			}
			return true
		})

		h.channels.Range(func(id interface{}, ci interface{}) bool {
			if removed[ci.(*channel).peer] {
				ci.(*channel).Close()
				h.channels.Delete(id)
				ci.(*channel).Swarm().channels.Delete(id)
			}
			return true
		})

		for p := range removed {
			h.scheduler.RemovePeer(p)
		}
	}
}

func (h *Host) handleMemeRequest(w *MemeWriter, r *MemeRequest) {
	if len(r.Messages) == 0 {
		return
	}

	channel, err := h.channel(r)
	if err != nil {
		log.Println("failed creating channel", err)
		return
	}
	channel.HandleMemeRequest(w, r)
}

func (h *Host) peer(c TransportConn) (peer *Peer) {
	pi, ok := h.peers.Load(c.String())
	if ok {
		return pi.(*Peer)
	}

	pi, loaded := h.peers.LoadOrStore(c.String(), NewPeer(h.nextUID(), c))
	peer = pi.(*Peer)
	if !loaded {
		h.scheduler.AddPeer(h.ctx, peer)
	}
	return
}

func (h *Host) channel(r *MemeRequest) (c *channel, err error) {
	ci, ok := h.channels.Load(r.ChannelID)
	if ok {
		return ci.(*channel), nil
	}

	sid, err := readHandshakeRequest(r)
	if err != nil {
		return
	}
	si, ok := h.swarms.Load(sid.String())
	if !ok {
		return nil, errors.New("handshake: unknown swarm")
	}

	peer := h.peer(r.Conn)

	for {
		cid := uint32(rand.Int63n(math.MaxUint32))
		c = newChannel(channelOptions{
			ID:    cid,
			Swarm: si.(*Swarm),
			Peer:  peer,
			Conn:  r.Conn,
		})
		_, ok = h.channels.LoadOrStore(cid, c)
		if !ok {
			peer.channels.Store(cid, c)
			si.(*Swarm).channels.Store(cid, c)
			return
		}
	}
}

func readHandshakeRequest(r *MemeRequest) (sid *SwarmID, err error) {
	if r.ChannelID != 0 {
		return nil, errors.New("handshake: non-zero channel id")
	}
	if len(r.Messages) == 0 {
		return nil, errors.New("handshake: first message is empty")
	}

	hs, ok := r.Messages[0].(*Handshake)
	if !ok {
		return nil, errors.New("handshake: first message is not handshake")
	}
	swarmIDOptionIf, ok := hs.Options.Find(SwarmIdentifierOption)
	if !ok {
		return nil, errors.New("handshake: handshake has no swarm id")
	}

	sid = NewSwarmID(*swarmIDOptionIf.(*SwarmIdentifierProtocolOption))
	return
}

// // StreamSwarmOptions ...
// type StreamSwarmOptions struct {
// 	id    []byte
// 	queue *pubsub.Queue
// }
