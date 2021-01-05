// +build js

package wasmio

import (
	"sync"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/iotime"
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
	p.proxy = api.Call("openBus", label, proxy)

	return p
}

// newBusFromProxy ...
func newBusFromProxy(v js.Value) (*Bus, js.Value) {
	p := &Bus{
		proxy: v,
		q:     make(chan []byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))

	return p, proxy
}

// Bus ...
type Bus struct {
	proxy  js.Value
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

	data := jsUint8Array.New(len(b))
	js.CopyBytesToJS(data, b)
	p.proxy.Call("write", data)
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
	return
}

// Close ...
func (p *Bus) Close() error {
	p.closeWithError(ErrClosed)
	p.proxy.Call("close")
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
	iotime.Store(int64(args[2].Float()))

	n := args[1].Int()
	b := busBuffers.Get().([]byte)

	if n > cap(b) {
		b = make([]byte, n)
	}
	b = b[:n]

	js.CopyBytesToGo(b, args[0])
	p.q <- b
	return nil
}

func (p *Bus) onClose(this js.Value, args []js.Value) interface{} {
	p.closeWithError(ErrClosed)
	return nil
}
