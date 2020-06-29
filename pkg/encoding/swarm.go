package encoding

import (
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/encoding/codec"
	"github.com/MemeLabs/go-ppspp/pkg/encoding/store"
)

// SwarmOptions ...
type SwarmOptions struct {
	ChunkSize  int
	LiveWindow int
}

func (o *SwarmOptions) assign(u SwarmOptions) {
	if u.ChunkSize != 0 {
		o.ChunkSize = u.ChunkSize
	}

	if u.LiveWindow != 0 {
		o.LiveWindow = u.LiveWindow
	}
}

// NewDefaultSwarmOptions ...
func NewDefaultSwarmOptions() SwarmOptions {
	return SwarmOptions{
		ChunkSize:  1024,
		LiveWindow: 1 << 16,
	}
}

// NewDefaultSwarm ...
func NewDefaultSwarm(id SwarmID) (s *Swarm) {
	s, _ = NewSwarm(id, NewDefaultSwarmOptions())
	return
}

// NewSwarm ...
func NewSwarm(id SwarmID, opt SwarmOptions) (*Swarm, error) {
	o := NewDefaultSwarmOptions()
	o.assign(opt)

	buf, err := store.NewBuffer(o.LiveWindow, o.ChunkSize)
	if err != nil {
		return nil, err
	}

	requestedBins := &loadingBins{
		bins: binmap.New(),
	}

	pubSub := &store.PubSub{}
	pubSub.Subscribe(buf)
	pubSub.Subscribe(requestedBins)

	return &Swarm{
		id:            id,
		options:       o,
		channels:      map[*Peer]*channel{},
		store:         buf,
		pubSub:        pubSub,
		requestedBins: requestedBins,
	}, nil
}

type loadingBins struct {
	sync.Mutex
	bins *binmap.Map
}

func (b *loadingBins) Consume(c store.Chunk) {
	b.Lock()
	defer b.Unlock()
	b.bins.Set(c.Bin)
}

// Swarm ...
type Swarm struct {
	id            SwarmID
	options       SwarmOptions
	channelsLock  sync.Mutex
	channels      map[*Peer]*channel
	store         *store.Buffer
	pubSub        *store.PubSub
	requestedBins *loadingBins // bins we have or have requested
}

// ID ...
func (s *Swarm) ID() SwarmID {
	return s.id
}

// URI ...
func (s *Swarm) URI() *URI {
	return &URI{
		ID: s.ID(),
		Options: URIOptions{
			codec.ChunkSizeOption: s.chunkSize(),
		},
	}
}

func (s *Swarm) chunkSize() int {
	return s.options.ChunkSize
}

func (s *Swarm) liveWindow() int {
	return s.options.LiveWindow
}

func (s *Swarm) loadedBins() *binmap.Map {
	return s.store.Bins()
}

func (s *Swarm) addChannel(p *Peer, c *channel) {
	s.channelsLock.Lock()
	defer s.channelsLock.Unlock()

	s.pubSub.Subscribe(c)
	s.channels[p] = c
}

func (s *Swarm) removeChannel(p *Peer) {
	s.channelsLock.Lock()
	defer s.channelsLock.Unlock()

	if c, ok := s.channels[p]; ok {
		s.pubSub.Unsubscribe(c)
		delete(s.channels, p)
	}
}

// Reader ...
func (s *Swarm) Reader() *store.Reader {
	return s.store.Reader()
}

// Close ...
func (s *Swarm) Close() error {
	// * make sure we've sent at least 1 of every bin...?
	// * graceful shutdown deadline

	s.channelsLock.Lock()
	defer s.channelsLock.Unlock()

	for p, c := range s.channels {
		p.removeChannel(s)
		c.Close()
	}

	return nil
}
