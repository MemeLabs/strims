// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ioutil

import (
	"errors"
	"io"
	"sync"
)

var (
	ErrNewWriteFlushSamplerBusy   = errors.New("write sampler busy")
	ErrNewWriteFlushSamplerClosed = errors.New("write sampler closed")
)

const (
	sampleStateClean = iota
	sampleStateDirty
	sampleStateWaiting
	sampleStateReading
	sampleStateClosed
)

func NewWriteFlushSampler(w WriteFlusher) *WriteFlushSampler {
	return &WriteFlushSampler{
		w:    w,
		errs: make(chan error),
	}
}

type WriteFlushSampler struct {
	mu    sync.Mutex
	state int
	errs  chan error
	sw    io.Writer
	w     WriteFlusher
}

func (w *WriteFlushSampler) Write(p []byte) (int, error) {
	w.mu.Lock()
	state := w.state
	if state == sampleStateClean {
		w.state = sampleStateDirty
	}
	w.mu.Unlock()

	if state == sampleStateReading {
		if _, err := w.sw.Write(p); err != nil {
			w.errs <- err

			w.mu.Lock()
			w.state = sampleStateDirty
			w.sw = nil
			w.mu.Unlock()
		}
	}

	return w.w.Write(p)
}

func (w *WriteFlushSampler) Flush() error {
	w.mu.Lock()
	switch w.state {
	case sampleStateWaiting:
		w.state = sampleStateReading
	case sampleStateReading:
		w.errs <- nil
		w.sw = nil
		fallthrough
	default:
		w.state = sampleStateClean
	}
	w.mu.Unlock()

	return w.w.Flush()
}

func (w *WriteFlushSampler) Close() error {
	w.mu.Lock()
	defer w.mu.Unlock()

	switch w.state {
	case sampleStateClosed:
		return ErrNewWriteFlushSamplerClosed
	default:
		w.state = sampleStateClosed
		close(w.errs)
	}

	return nil
}

func (w *WriteFlushSampler) Sample(sw io.Writer) error {
	w.mu.Lock()
	switch w.state {
	case sampleStateClean:
		w.state = sampleStateReading
	case sampleStateDirty:
		w.state = sampleStateWaiting
	case sampleStateWaiting:
		fallthrough
	case sampleStateReading:
		return ErrNewWriteFlushSamplerBusy
	case sampleStateClosed:
		return ErrNewWriteFlushSamplerClosed
	}
	w.sw = sw
	w.mu.Unlock()

	err, ok := <-w.errs
	if !ok {
		return ErrNewWriteFlushSamplerClosed
	}
	return err
}
