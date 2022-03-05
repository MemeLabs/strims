//go:build js

package wasmio

import (
	"io"
	"sync"
	"syscall/js"
)

var busBuffers = busBufferPool{
	sync.Pool{
		New: func() interface{} {
			b := make([]byte, 1024)
			return &b
		},
	},
}

type busBufferPool struct {
	p sync.Pool
}

func (p *busBufferPool) Get(n int) *[]byte {
	b := p.p.Get().(*[]byte)
	if n > cap(*b) {
		*b = make([]byte, n)
	}
	*b = (*b)[:n]
	return b
}

func (p *busBufferPool) Put(b *[]byte) {
	p.p.Put(b)
}

// NewBus ...
func NewBus(api js.Value, label string) Bus {
	p := &channel{
		pool: &busBuffers,
		q:    make(chan *[]byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))
	p.id = api.Call("openBus", label, proxy).Int()

	return p
}

// newBusFromProxy ...
func newBusFromProxy(id int) (Bus, js.Value) {
	p := &channel{
		pool: &busBuffers,
		id:   id,
		q:    make(chan *[]byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))

	return p, proxy
}

// Bus ...
type Bus interface {
	io.ReadWriteCloser
}
