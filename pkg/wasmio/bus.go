// +build js

package wasmio

import (
	"sync"
	"syscall/js"
)

var busBuffers = sync.Pool{
	New: func() interface{} {
		return make([]byte, 1024)
	},
}

// NewBus ...
func NewBus(api js.Value, label string) *Bus {
	p := &Bus{
		q: make(chan []byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))
	p.id = api.Call("openBus", label, proxy).Int()

	return p
}

// newBusFromProxy ...
func newBusFromProxy(id int) (*Bus, js.Value) {
	p := &Bus{
		id: id,
		q:  make(chan []byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))

	return p, proxy
}

// Bus ...
type Bus struct {
	id     int
	funcs  Funcs
	q      chan []byte
	b      []byte
	off    int
	closed bool
	err    error
}

// Write ...
func (p *Bus) Write(b []byte) (int, error) {
	if p.closed {
		return 0, p.err
	}

	t, ok := channelWrite(p.id, b)
	if !ok {
		return 0, ErrClosed
	}

	syncTime(t)
	return len(b), nil
}

// Read ...
func (p *Bus) Read(b []byte) (n int, err error) {
	if p.b == nil || p.off == len(p.b) {
		if p.b != nil {
			busBuffers.Put(p.b)
		}

		b, ok := <-p.q
		if !ok {
			return 0, p.err
		}

		p.b = b
		p.off = 0
	}

	n = len(p.b) - p.off
	if n > len(b) {
		n = len(b)
	}

	copy(b[:n], p.b[p.off:])
	p.off += n
	return n, p.err
}

// Close ...
func (p *Bus) Close() error {
	p.closeWithError(ErrClosed)
	channelClose(p.id)
	p.funcs.Release()
	return nil
}

func (p *Bus) closeWithError(err error) {
	if !p.closed {
		p.closed = true
		p.err = err
		close(p.q)
	}
}

func (p *Bus) onData(this js.Value, args []js.Value) interface{} {
	n := args[0].Int()
	b := busBuffers.Get().([]byte)

	if n > cap(b) {
		b = make([]byte, n)
	}
	b = b[:n]

	t, ok := channelRead(p.id, b)
	if !ok {
		return ErrClosed
	}

	syncTime(t)
	p.q <- b
	return nil
}

func (p *Bus) onClose(this js.Value, args []js.Value) interface{} {
	p.closeWithError(ErrClosed)
	return nil
}
