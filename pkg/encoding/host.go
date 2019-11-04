package encoding

import (
	"context"
	"errors"
	"log"
	"math"
	"math/rand"
	"sync"
	"sync/atomic"
	"time"

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

type joinThing struct {
	uri   TransportURI
	swarm *Swarm
}

// NewHost ...
func NewHost(o *HostOptions) (c *Host) {
	c = &Host{
		ctx:       o.Context,
		scheduler: NewScheduler(o.Context),
		// peerThing:  NewPeerThing(),
		transports: map[string]Transport{},
		joinThings: make(chan joinThing, 64),
	}

	c.servers = make([]*MemeServer, len(o.Transports))
	for i, t := range o.Transports {
		c.servers[i] = &MemeServer{
			t:       t,
			Handler: c.handleMemeRequest,
		}

		c.transports[t.Scheme()] = t
	}

	// TODO: hax
	go c.doJoinThing()

	return c
}

// Host ...
type Host struct {
	sync.Mutex

	ctx        context.Context
	prevUID    int64
	servers    []*MemeServer
	transports map[string]Transport
	swarms     sync.Map
	peers      sync.Map
	channels   sync.Map
	scheduler  *Scheduler
	// peerThing  *PeerThing
	joinThings chan joinThing
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
	// TODO: hax
	s.joinThings = h.joinThings
	h.swarms.Store(s.ID.String(), s)
}

func (h *Host) doJoinThing() {
	for {
		select {
		case <-h.ctx.Done():
			return
		case t := <-h.joinThings:
			if err := h.joinSwarm(t.swarm, t.uri); err != nil {
				// debug.Red(err, t.uri)
			}
		}
	}
}

func (h *Host) joinSwarm(s *Swarm, uri TransportURI) (err error) {
	p, err := h.dialPeer(uri)
	if err != nil {
		return err
	}

	found := false
	p.channels.Range(func(_ interface{}, ci interface{}) bool {
		found = ci.(*channel).swarm == s
		return !found
	})
	if found {
		return errors.New("this peer already has a channel for this swarm")
	}

	p.Lock()
	defer p.Unlock()

	cid := uint32(rand.Int63n(math.MaxUint32))
	ch := newChannel(channelOptions{
		ID:    cid,
		Swarm: s,
		Peer:  p,
		Conn:  p.conn,
	})
	h.channels.Store(cid, ch)
	p.channels.Store(cid, ch)
	s.channels.Store(cid, ch)

	w := NewMemeWriter(p.conn)
	ch.OfferHandshake(w)
	w.Flush()

	return nil
}

// ErrUnsupportedTransport ...
var ErrUnsupportedTransport = errors.New("Unsupported transport")

// JoinSwarm ...
func (h *Host) JoinSwarm(id *SwarmID, uri TransportURI) (r *ChunkBufferReader, err error) {
	s := NewDefaultSwarm(id)
	// TODO: hax
	s.joinThings = h.joinThings

	h.swarms.Store(id.String(), s)

	if err = h.joinSwarm(s, uri); err != nil {
		return
	}

	r = s.chunks.Reader()
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
	for i := range h.servers {
		go func(i int) {
			if err := h.servers[i].Listen(ctx); err != nil {
				log.Println("server error", err)
			}
			cancel()
			wg.Done()
		}(i)
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

func (h *Host) dialPeer(uri TransportURI) (p *Peer, err error) {
	pi, ok := h.peers.Load(uri)
	if ok {
		return pi.(*Peer), nil
	}

	t, ok := h.transports[uri.Scheme()]
	if !ok {
		return nil, ErrUnsupportedTransport
	}

	c, err := t.Dial(uri)
	if err != nil {
		return nil, err
	}

	return h.storePeer(c)
}

func (h *Host) storePeer(c TransportConn) (p *Peer, err error) {
	uri := c.URI()
	pi, ok := h.peers.Load(uri)
	if ok {
		c.Close()
		return pi.(*Peer), nil
	}

	pi, loaded := h.peers.LoadOrStore(uri, NewPeer(h.nextUID(), c))
	p = pi.(*Peer)
	if loaded {
		c.Close()
	} else {
		h.scheduler.AddPeer(h.ctx, p)
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

	peer, err := h.storePeer(r.Conn)
	if err != nil {
		return nil, err
	}

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
