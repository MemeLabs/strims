// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/MemeLabs/strims/pkg/ioutil"
)

// Publisher ...
type Publisher interface {
	Publish(Chunk)
	Reset()
}

// NewWriter ...
func NewWriter(pub Publisher, chunkSize int) ioutil.WriteFlushResetter {
	return bufioutil.NewWriter(&writer{pub: pub}, chunkSize)
}

// writer assigns addresses to chunks
type writer struct {
	pub Publisher
	bin binmap.Bin
}

// Write ...
func (w *writer) Write(p []byte) (n int, err error) {
	w.pub.Publish(Chunk{w.bin, p})
	w.bin += 2
	return len(p), nil
}

// Reset ...
func (w *writer) Reset() {
	w.bin = 0
	w.pub.Reset()
}
