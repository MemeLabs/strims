// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"errors"
	"fmt"

	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
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

	v, err := integrity.NewVerifier(id, o.IntegrityVerifierOptions())
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
	reader   *store.BufferReader
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
func (s *Swarm) Reader() *store.BufferReader {
	return store.NewBufferReader(s.store)
}

func (s *Swarm) ImportCache(c *swarmpb.Cache) error {
	if c.Uri != s.URI().String() {
		return errors.New("cache import failed: incompatible swarm options")
	}
	if err := s.verifier.ImportCache(c); err != nil {
		return fmt.Errorf("cache import failed: %w", err)
	}
	if err := s.store.ImportCache(c.Data); err != nil {
		return fmt.Errorf("cache import failed: %w", err)
	}
	return nil
}

func (s *Swarm) ExportCache() (*swarmpb.Cache, error) {
	data, err := s.store.ExportCache()
	if err != nil {
		return nil, fmt.Errorf("cache export failed: %w", err)
	}

	return &swarmpb.Cache{
		Uri:       s.URI().String(),
		Integrity: s.verifier.ExportCache(),
		Data:      data,
	}, nil
}

// Close ...
func (s *Swarm) Close() error {
	// * make sure we've sent at least 1 of every bin...?
	// * graceful shutdown deadline

	s.store.Close()

	return nil
}
