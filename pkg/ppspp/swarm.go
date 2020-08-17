package ppspp

import (
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ma"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
)

// SwarmOptions ...
type SwarmOptions struct {
	ChunkSize          int
	ChunksPerSignature int
	LiveWindow         int
	Integrity          integrity.VerifierOptions
}

// Assign ...
func (o *SwarmOptions) Assign(u SwarmOptions) {
	if u.ChunkSize != 0 {
		o.ChunkSize = u.ChunkSize
	}
	if u.ChunksPerSignature != 0 {
		o.ChunksPerSignature = u.ChunksPerSignature
	}
	if u.LiveWindow != 0 {
		o.LiveWindow = u.LiveWindow
	}

	o.Integrity.Assign(u.Integrity)
}

// NewDefaultSwarmOptions ...
func NewDefaultSwarmOptions() SwarmOptions {
	return SwarmOptions{
		ChunkSize:          1024,
		ChunksPerSignature: 32,
		LiveWindow:         1 << 16,
		Integrity:          integrity.NewDefaultVerifierOptions(),
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
	o.Assign(opt)

	buf, err := store.NewBuffer(o.LiveWindow, o.ChunkSize)
	if err != nil {
		return nil, err
	}

	bins := &swarmBins{
		Requested:   binmap.New(),
		Available:   binmap.New(),
		binRateFast: ma.NewSimple(30, 100*time.Millisecond),
		binRateSlow: ma.NewSimple(300, 100*time.Millisecond),
	}

	vo := integrity.SwarmVerifierOptions{
		LiveDiscardWindow:  o.LiveWindow,
		ChunkSize:          o.ChunkSize,
		ChunksPerSignature: o.ChunksPerSignature,
		VerifierOptions:    o.Integrity,
	}
	v, err := integrity.NewVerifier(id.Binary(), vo)
	if err != nil {
		return nil, err
	}

	return &Swarm{
		id:       id,
		options:  o,
		channels: map[*Peer]*channel{},
		store:    buf,
		pubSub:   store.NewPubSub(buf, bins),
		bins:     bins,
		verifier: v,
	}, nil
}

type swarmBins struct {
	sync.Mutex
	Requested *binmap.Map // bins we have or have requested
	Available *binmap.Map // bins at least one peer has

	binRateFast   ma.Simple
	binRateSlow   ma.Simple
	lastAvailable binmap.Bin
	lastTime      time.Time
}

func (s *swarmBins) ResetRequested(b binmap.Bin) {
	s.Lock()
	defer s.Unlock()
	s.Requested.Reset(b)
}

func (s *swarmBins) AddAvailable(b binmap.Bin) {
	s.Lock()
	defer s.Unlock()
	s.Available.Set(b)

	br := b.BaseRight()
	if s.lastAvailable > br {
		return
	}

	t := iotime.Load()

	if s.lastAvailable == 0 {
		s.lastAvailable = br
		s.lastTime = t
		return
	}

	d := uint64((br - s.lastAvailable) / 2)
	s.binRateFast.AddWithTime(d, t)
	s.binRateSlow.AddWithTime(d, t)
	s.lastAvailable = br

	// TODO: compute rate and time from timestamp in munro signatures?
	td := time.Duration(d) * s.binRateSlow.IntervalWithTime(t)
	et := s.lastTime.Add(td)
	s.lastTime = s.lastTime.Add(t.Sub(et) / 2)
}

func (s *swarmBins) estEndBinWithTime(t time.Time) binmap.Bin {
	d := t.Sub(s.lastTime)
	ivl := s.binRateSlow.Interval()
	if d == 0 || ivl == 0 {
		return s.lastAvailable
	}
	return s.lastAvailable + binmap.Bin(d/ivl)*2
}

func (s *swarmBins) Consume(c store.Chunk) bool {
	s.Lock()
	defer s.Unlock()
	s.Requested.Set(c.Bin)
	return true
}

// Swarm ...
type Swarm struct {
	id           SwarmID
	options      SwarmOptions
	channelsLock sync.Mutex
	channels     map[*Peer]*channel
	store        *store.Buffer
	pubSub       *store.PubSub
	bins         *swarmBins
	verifier     integrity.SwarmVerifier
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
			codec.ChunkSizeOption:                        s.chunkSize(),
			codec.ChunksPerSignatureOption:               s.chunksPerSignature(),
			codec.ContentIntegrityProtectionMethodOption: int(s.contentIntegrityProtectionMethod()),
			codec.MerkleHashTreeFunctionOption:           int(s.merkleHashTreeFunction()),
			codec.LiveSignatureAlgorithmOption:           int(s.liveSignatureAlgorithm()),
		},
	}
}

func (s *Swarm) chunkSize() int {
	return s.options.ChunkSize
}

func (s *Swarm) chunksPerSignature() int {
	return s.options.ChunksPerSignature
}

func (s *Swarm) liveWindow() int {
	return s.options.LiveWindow
}

func (s *Swarm) contentIntegrityProtectionMethod() integrity.ProtectionMethod {
	return s.options.Integrity.ProtectionMethod
}

func (s *Swarm) merkleHashTreeFunction() integrity.MerkleHashTreeFunction {
	return s.options.Integrity.MerkleHashTreeFunction
}

func (s *Swarm) liveSignatureAlgorithm() integrity.LiveSignatureAlgorithm {
	return s.options.Integrity.LiveSignatureAlgorithm
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

func (s *Swarm) removeChannel(p *Peer) []*channel {
	s.channelsLock.Lock()
	defer s.channelsLock.Unlock()

	cs := []*channel{}
	if c, ok := s.channels[p]; ok {
		cs = append(cs, c)
		s.pubSub.Unsubscribe(c)
		delete(s.channels, p)
	}
	return cs
}

// Reader ...
func (s *Swarm) Reader() *store.Buffer {
	return s.store
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
