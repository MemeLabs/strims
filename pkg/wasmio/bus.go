package wasmio

import (
	"sync"
	"syscall/js"
)

// NewBus ...
func NewBus(v js.Value) *Bus {
	ch := &Bus{
		v: v,
		d: v.Get("data"),
		q: make(chan []byte, 16),
	}
	v.Set("onWrite", js.FuncOf(ch.handleWrite).Value)
	return ch
}

var readBuffers = sync.Pool{
	New: func() interface{} {
		return make([]byte, 1024)
	},
}

// Bus ...
type Bus struct {
	v   js.Value
	d   js.Value
	off int
	b   []byte
	q   chan []byte
}

func (c *Bus) handleWrite(this js.Value, args []js.Value) interface{} {
	n := args[0].Int()
	b := readBuffers.Get().([]byte)

	if int(n) > cap(b) {
		b = make([]byte, n)
	}
	b = b[:n]

	js.CopyBytesToGo(b, c.d)
	c.q <- b

	return nil
}

// Write ...
func (c *Bus) Write(p []byte) (n int, err error) {
	n = js.CopyBytesToJS(c.d, p)
	c.v.Call("emitData", js.ValueOf(n))
	return
}

// Read ...
func (c *Bus) Read(p []byte) (n int, err error) {
	if c.b == nil || c.off == len(c.b) {
		if c.b != nil {
			readBuffers.Put(c.b)
		}

		c.b = <-c.q
		c.off = 0
	}

	n = len(c.b) - c.off
	if n > len(p) {
		n = len(p)
	}

	copy(p[:n], c.b[c.off:])
	c.off += n
	return
}
