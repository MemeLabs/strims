package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
)

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

	v, err := integrity.NewVerifier(id.Binary(), o.IntegrityVerifierOptions())
	if err != nil {
		return nil, err
	}

	return &Swarm{
		id:       id,
		options:  o,
		store:    buf,
		pubSub:   store.NewPubSub(buf),
		verifier: v,
	}, nil
}

// Swarm ...
type Swarm struct {
	id       SwarmID
	options  SwarmOptions
	store    *store.Buffer
	pubSub   *store.PubSub
	verifier integrity.SwarmVerifier
}

// ID ...
func (s *Swarm) ID() SwarmID {
	return s.id
}

// URI ...
func (s *Swarm) URI() *URI {
	return &URI{
		ID:      s.ID(),
		Options: s.options.URIOptions(),
	}
}

// Reader ...
func (s *Swarm) Reader() *store.Buffer {
	return s.store
}

// Close ...
func (s *Swarm) Close() error {
	// * make sure we've sent at least 1 of every bin...?
	// * graceful shutdown deadline

	s.store.Close()

	return nil
}
