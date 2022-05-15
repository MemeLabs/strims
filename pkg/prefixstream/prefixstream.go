// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package prefixstream

import (
	"bytes"
	"encoding/binary"
	"io"

	"github.com/MemeLabs/strims/pkg/ioutil"
)

var sentinel = [...]byte{0, 1, 1, 2, 3, 5, 8, 13}

const sentinelLen = len(sentinel)

// NewWriter ...
func NewWriter(w io.Writer) *Writer {
	x := &Writer{w: w}
	copy(x.h[:], sentinel[:])
	return x
}

// Writer ...
type Writer struct {
	w io.Writer
	h [sentinelLen + binary.MaxVarintLen32]byte
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	n := sentinelLen
	n += binary.PutUvarint(w.h[n:], uint64(len(p)))

	if _, err := w.w.Write(w.h[:n]); err != nil {
		return 0, err
	}
	if _, err := w.w.Write(p); err != nil {
		return 0, err
	}

	return n + len(p), nil
}

// NewReader ...
func NewReader(r io.Reader) *Reader {
	return &Reader{r: r}
}

// Reader ...
type Reader struct {
	r  io.Reader
	h  bytes.Buffer
	hn int
	n  int
}

// Read ...
func (r *Reader) Read(p []byte) (int, error) {
	if err := r.readHeader(); err != nil {
		return 0, err
	}

	if r.n < len(p) {
		p = p[:r.n]
	}
	n, err := io.ReadFull(r.r, p)
	if err != nil {
		return 0, err
	}

	r.n -= n
	if r.n == 0 {
		err = io.EOF
	}

	return n, err
}

func (r *Reader) readHeader() error {
	for r.n == 0 {
		if _, err := io.CopyN(&r.h, r.r, int64(r.hn)); err != nil {
			return err
		}

		i := bytes.Index(r.h.Bytes(), sentinel[:])
		if i == -1 {
			r.hn = 1
			continue
		}
		r.h.Reset()

		n, err := binary.ReadUvarint(ioutil.NewByteReader(r.r))
		if err != nil {
			return err
		}
		r.n = int(n)
		r.hn = sentinelLen
	}
	return nil
}
