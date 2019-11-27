package encoding

import (
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// SwarmOptions ...
type SwarmOptions struct {
	// ChunkSize  int
	LiveWindow int
}

// NewDefaultSwarmOptions ...
func NewDefaultSwarmOptions() SwarmOptions {
	return SwarmOptions{
		// ChunkSize:  1024,    // this isn't actually configurable...
		// LiveWindow: 1 << 14, // 16MB
		LiveWindow: 1 << 16, // 64MB
	}
}

// NewDefaultSwarm ...
func NewDefaultSwarm(id *SwarmID) (s *Swarm) {
	s, _ = NewSwarm(id, NewDefaultSwarmOptions())
	return
}

// NewSwarm ...
func NewSwarm(id *SwarmID, o SwarmOptions) (s *Swarm, err error) {
	s = &Swarm{
		ID: id,
		// ChunkSize:     o.ChunkSize,
		LiveWindow:     o.LiveWindow,
		loadedBins:     binmap.New(),
		requestedBins:  binmap.New(),
		peerCandidates: newPeerCandidateMap(),
	}
	s.chunks, err = newChunkBuffer(o.LiveWindow)
	return
}

type peerCandidateMap struct {
	l sync.Mutex
	m map[TransportURI]peerCandidate
}

type peerCandidate struct {
	URI      TransportURI
	LastSeen time.Time
}

func newPeerCandidateMap() *peerCandidateMap {
	return &peerCandidateMap{
		m: map[TransportURI]peerCandidate{},
	}
}

func (m *peerCandidateMap) AddURI(u TransportURI) {
	m.l.Lock()
	defer m.l.Unlock()

	m.m[u] = peerCandidate{
		URI:      u,
		LastSeen: time.Now(),
	}
	// send to channel...?
}

// Swarm ...
type Swarm struct {
	sync.Mutex

	ID *SwarmID
	// ChunkSize  int
	LiveWindow int

	channels        sync.Map
	chunks          *chunkBuffer
	firstRequestBin binmap.Bin
	loadedBins      *binmap.Map
	requestedBins   *binmap.Map
	// TODO: hax
	joinThings     chan joinThing
	peerCandidates *peerCandidateMap
}

func (s *Swarm) AddPeerCandidate(t TransportURI) {

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

	// TODO: this is stored twice, once  here and once in chunks... remove this
	s.loadedBins.Set(b)

	// TODO: prevents server from requesting loaded bins... rename
	s.requestedBins.Set(b)
	s.Unlock()

	s.channels.Range(func(id interface{}, ci interface{}) bool {
		c := ci.(*channel)
		c.Lock()
		defer c.Unlock()

		c.addedBins.Set(b)
		return true
	})
}

// Leave ...
func (s *Swarm) Leave() error {
	// * choke channels
	// * make sure we've sent at least 1 of every bin...?
	// * close channels
	// * graceful shutdown deadline
	return nil
}
