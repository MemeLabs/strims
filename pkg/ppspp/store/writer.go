// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
)

// Publisher ...
type Publisher interface {
	Publish(Chunk)
}

// NewWriter ...
func NewWriter(pub Publisher, chunkSize int) *Writer {
	return &Writer{
		bw: bufioutil.NewWriter(&writer{pub: pub}, chunkSize),
	}
}

// Writer ...
type Writer struct {
	bw *bufioutil.Writer
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	return w.bw.Write(p)
}

// Flush ...
func (w *Writer) Flush() error {
	return w.bw.Flush()
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
