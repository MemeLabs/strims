// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
)

// WriterOptions ...
type WriterOptions struct {
	SwarmOptions SwarmOptions
	Key          *key.Key
}

// NewWriter ...
func NewWriter(o WriterOptions) (*Writer, error) {
	so := SwarmOptions{
		SchedulingMethod: SeedSchedulingMethod,
	}
	so.Assign(o.SwarmOptions)

	id := NewSwarmID(o.Key.Public)
	s, err := NewSwarm(id, so)
	if err != nil {
		return nil, err
	}

	s.store.SetOffset(0)

	sw := store.NewWriter(s.pubSub, s.options.ChunkSize)
	w, err := integrity.NewWriter(o.Key.Private, s.verifier, sw, s.options.IntegrityWriterOptions())
	if err != nil {
		return nil, err
	}

	return &Writer{
		w: w,
		s: s,
	}, nil
}

// Writer ...
type Writer struct {
	w ioutil.WriteFlusher
	s *Swarm
}

// Swarm ...
func (w *Writer) Swarm() *Swarm {
	return w.s
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	return w.w.Write(p)
}

// Flush ...
func (w *Writer) Flush() error {
	return w.w.Flush()
}

// Close shut down the swarm...
func (w *Writer) Close() (err error) {
	return w.s.Close()
}
