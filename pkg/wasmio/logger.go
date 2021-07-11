//go:build js
// +build js

package wasmio

import (
	"bytes"
	"sync"
	"syscall/js"
)

// NewZapSink ...
func NewZapSink(bridge js.Value) *ZapSink {
	return &ZapSink{
		bridge: bridge,
	}
}

// ZapSink ...
type ZapSink struct {
	bridge  js.Value
	bufLock sync.Mutex
	buf     bytes.Buffer
}

// Write ...
func (s *ZapSink) Write(p []byte) (int, error) {
	s.bufLock.Lock()
	defer s.bufLock.Unlock()

	// return s.buf.Write(p)

	data := jsUint8Array.New(len(p))
	js.CopyBytesToJS(data, p)
	s.bridge.Call("syncLogs", data)

	return len(p), nil
}

// Sync ...
func (s *ZapSink) Sync() error {
	s.bufLock.Lock()
	defer s.bufLock.Unlock()

	data := jsUint8Array.New(s.buf.Len())
	js.CopyBytesToJS(data, s.buf.Bytes())
	s.bridge.Call("syncLogs", data)
	return nil
}

// Close ...
func (s *ZapSink) Close() error {
	s.bufLock.Lock()
	defer s.bufLock.Unlock()

	return nil
}
