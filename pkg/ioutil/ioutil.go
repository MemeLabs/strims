// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ioutil

import "io"

// Flusher ...
type Flusher interface {
	Flush() error
}

// WriteFlusher ...
type WriteFlusher interface {
	io.Writer
	Flusher
}

// WriteFlushCloser ...
type WriteFlushCloser interface {
	io.Closer
	WriteFlusher
}

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.ReadWriter
	Flusher
}

type BufferedWriteFlusher interface {
	WriteFlusher
	Available() int
	AvailableBuffer() []byte
}
