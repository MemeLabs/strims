package encoding

import (
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// SwarmOptions ...
type SwarmOptions struct {
	ChunkSize  int
	LiveWindow int
}

func (o *SwarmOptions) applyDefaults() {
	if o.ChunkSize == 0 {
		o.ChunkSize = 1024
	}

	if o.LiveWindow == 0 {
		o.LiveWindow = 1 << 16 // 64MB
	}
}

// NewDefaultSwarmOptions ...
func NewDefaultSwarmOptions() (s SwarmOptions) {
	s.applyDefaults()
	return
}

// NewDefaultSwarm ...
func NewDefaultSwarm(id SwarmID) (s *Swarm) {
	s, _ = NewSwarm(id, NewDefaultSwarmOptions())
	return
}

// NewSwarm ...
func NewSwarm(id SwarmID, o SwarmOptions) (*Swarm, error) {
	o.applyDefaults()

	chunks, err := newChunkBuffer(o.LiveWindow, o.ChunkSize)
	if err != nil {
		return nil, err
	}

	return &Swarm{
		ID: id,
		URI: &URI{
			ID: id,
			Options: URIOptions{
				ChunkSizeOption:         o.ChunkSize,
				LiveDiscardWindowOption: o.LiveWindow,
			},
		},
		ChunkSize:     o.ChunkSize,
		LiveWindow:    o.LiveWindow,
		requestedBins: binmap.New(),
		chunks:        chunks,
	}, nil
}

// Swarm ...
type Swarm struct {
	sync.Mutex

	ID         SwarmID
	URI        *URI
	ChunkSize  int
	LiveWindow int

	channelsLock    sync.Mutex
	channels        channels
	chunks          *chunkBuffer
	firstRequestBin binmap.Bin
	requestedBins   *binmap.Map
}

// if chunks and requestedBins locked swarms wouldn't need a lock...?

// WriteChunk ...
func (s *Swarm) WriteChunk(b binmap.Bin, d []byte) {
	// TODO: this violates the convention where callers hold locks while using
	// swarms/channels/peers because otherwise we violate the lock order by
	// taking swarm's before channel's... find somewhere else to update channels.
	// maybe an injector like the js version... probably a writer in gospeak

	s.Lock()
	s.chunks.Set(b, d)

	// TODO: prevents server from requesting loaded bins... rename
	s.requestedBins.Set(b)
	s.Unlock()

	s.channelsLock.Lock()
	for _, c := range s.channels {
		c.Lock()
		c.addedBins.Set(b)
		c.Unlock()
	}
	s.channelsLock.Unlock()
}

// ReadChannel ...
func (s *Swarm) ReadChannel(p *Peer, l ReadWriteFlusher) Channel {
	ch := newChannel(channelOptions{
		Swarm: s,
		Peer:  p,
		Conn:  l,
	})

	p.Lock()
	p.channels.Insert(ch)
	p.Unlock()

	s.channelsLock.Lock()
	s.channels.Insert(ch)
	s.channelsLock.Unlock()

	ch.Lock()
	ch.OfferHandshake()
	ch.Unlock()

	go func() {
		<-ch.Done()

		p.Lock()
		p.channels.Remove(ch)
		p.Unlock()

		s.channelsLock.Lock()
		s.channels.Remove(ch)
		s.channelsLock.Unlock()
	}()

	return ch
}

// Reader ...
func (s *Swarm) Reader() *ChunkBufferReader {
	return s.chunks.Reader()
}

// Leave ...
func (s *Swarm) Leave() error {
	// * choke channels
	// * make sure we've sent at least 1 of every bin...?
	// * close channels
	// * graceful shutdown deadline
	return nil
}
