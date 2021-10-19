package ioutil

import (
	"bytes"
	"errors"
	"sync/atomic"
)

var (
	ErrNewWriteFlushSamplerBusy   = errors.New("write sampler busy")
	ErrNewWriteFlushSamplerClosed = errors.New("write sampler closed")
)

const (
	sampleStateClean uint32 = iota
	sampleStateDirty
	sampleStateWaiting
	sampleStateReading
	sampleStateClosed
)

func NewWriteFlushSampler(w WriteFlusher) *WriteFlushSampler {
	return &WriteFlushSampler{
		w:   w,
		buf: &bytes.Buffer{},
		ch:  make(chan []byte),
	}
}

type WriteFlushSampler struct {
	w     WriteFlusher
	buf   *bytes.Buffer
	ch    chan []byte
	state uint32
}

func (w *WriteFlushSampler) Write(p []byte) (int, error) {
	for {
		state := atomic.LoadUint32(&w.state)
		switch state {
		case sampleStateClean:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateDirty) {
				continue
			}
		case sampleStateReading:
			w.buf.Write(p)
		}
		break
	}

	return w.w.Write(p)
}

func (w *WriteFlushSampler) Flush() error {
	for {
		state := atomic.LoadUint32(&w.state)
		switch state {
		case sampleStateWaiting:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateReading) {
				continue
			}
		case sampleStateReading:
			w.emitSample()
			fallthrough
		default:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateClean) {
				continue
			}
		}
		break
	}

	return w.w.Flush()
}

func (w *WriteFlushSampler) Close() error {
	for {
		state := atomic.LoadUint32(&w.state)
		switch state {
		case sampleStateClosed:
			return ErrNewWriteFlushSamplerClosed
		default:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateClosed) {
				continue
			}
			close(w.ch)
		}
		return nil
	}
}

func (w *WriteFlushSampler) emitSample() {
	w.ch <- w.buf.Bytes()
	w.buf = &bytes.Buffer{}
}

func (w *WriteFlushSampler) Sample() ([]byte, error) {
	for {
		state := atomic.LoadUint32(&w.state)
		switch state {
		case sampleStateClean:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateReading) {
				continue
			}
		case sampleStateDirty:
			if !atomic.CompareAndSwapUint32(&w.state, state, sampleStateWaiting) {
				continue
			}
		case sampleStateWaiting:
			fallthrough
		case sampleStateReading:
			return nil, ErrNewWriteFlushSamplerBusy
		case sampleStateClosed:
			return nil, ErrNewWriteFlushSamplerClosed
		}
		break
	}

	b, ok := <-w.ch
	if !ok {
		return nil, ErrNewWriteFlushSamplerClosed
	}
	return b, nil
}
