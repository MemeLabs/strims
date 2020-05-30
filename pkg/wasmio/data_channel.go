// +build js

package wasmio

import (
	"errors"
	"sync"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/iotime"
)

const dataChannelMTU = 16 * 1024

var dataChannelBuffers = sync.Pool{
	New: func() interface{} {
		return make([]byte, dataChannelMTU)
	},
}

// NewDataChannelProxy ...
func NewDataChannelProxy(bridge js.Value, id int) (*DataChannelProxy, error) {
	p := &DataChannelProxy{
		q: make(chan []byte, 16),
	}

	onOpen := func(this js.Value, args []js.Value) interface{} {
		p.q <- nil
		return nil
	}

	proxy := jsObject.New()
	proxy.Set("onopen", p.funcs.Register(js.FuncOf(onOpen)))
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))
	proxy.Set("onerror", p.funcs.Register(js.FuncOf(p.onError)))
	p.proxy = bridge.Call("openDataChannel", id, proxy)

	<-p.q

	return p, p.err
}

// DataChannelProxy ...
type DataChannelProxy struct {
	proxy  js.Value
	funcs  Funcs
	q      chan []byte
	b      []byte
	off    int
	closed bool
	err    error
}

// MTU ...
func (w *DataChannelProxy) MTU() int {
	return dataChannelMTU
}

// Write ...
func (p *DataChannelProxy) Write(b []byte) (int, error) {
	if p.closed {
		return 0, p.err
	}

	data := jsUint8Array.New(len(b))
	js.CopyBytesToJS(data, b)
	p.proxy.Call("write", data)
	return len(b), nil
}

// Read ...
func (p *DataChannelProxy) Read(b []byte) (n int, err error) {
	if p.b == nil || p.off == len(p.b) {
		if p.b != nil {
			dataChannelBuffers.Put(p.b)
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
func (p *DataChannelProxy) Close() error {
	p.closeWithError(ErrClosed)
	p.proxy.Call("close")
	p.funcs.Release()
	return nil
}

func (p *DataChannelProxy) closeWithError(err error) {
	if !p.closed {
		p.closed = true
		p.err = err
		close(p.q)
	}
}

func (p *DataChannelProxy) onData(this js.Value, args []js.Value) interface{} {
	iotime.Store(int64(args[2].Float()))
	n := args[1].Int()
	b := dataChannelBuffers.Get().([]byte)

	if n > cap(b) {
		b = make([]byte, n)
	}
	b = b[:n]

	js.CopyBytesToGo(b, args[0])
	p.q <- b
	return nil
}

func (p *DataChannelProxy) onClose(this js.Value, args []js.Value) interface{} {
	p.closeWithError(ErrClosed)
	return nil
}

func (p *DataChannelProxy) onError(this js.Value, args []js.Value) interface{} {
	p.closeWithError(errors.New(args[0].String()))
	return nil
}
