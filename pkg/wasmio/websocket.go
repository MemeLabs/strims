// +build js

package wasmio

import (
	"errors"
	"sync"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/iotime"
)

const webSocketMTU = 64 * 1024

var webSocketBuffers = sync.Pool{
	New: func() interface{} {
		return make([]byte, webSocketMTU)
	},
}

// NewWebSocketProxy ...
func NewWebSocketProxy(bridge js.Value, uri string) (*WebSocketProxy, error) {
	p := &WebSocketProxy{
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
	p.proxy = bridge.Call("openWebSocket", uri, proxy)

	<-p.q

	return p, p.err
}

// WebSocketProxy ...
type WebSocketProxy struct {
	proxy  js.Value
	funcs  Funcs
	q      chan []byte
	b      []byte
	off    int
	closed bool
	err    error
}

// MTU ...
func (w *WebSocketProxy) MTU() int {
	return webSocketMTU
}

// Write ...
func (p *WebSocketProxy) Write(b []byte) (int, error) {
	if p.closed {
		return 0, p.err
	}

	data := jsUint8Array.New(len(b))
	js.CopyBytesToJS(data, b)
	p.proxy.Call("write", data)
	return len(b), nil
}

// Read ...
func (p *WebSocketProxy) Read(b []byte) (n int, err error) {
	if p.b == nil || p.off == len(p.b) {
		if p.b != nil {
			webSocketBuffers.Put(p.b)
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
func (p *WebSocketProxy) Close() error {
	p.closeWithError(ErrClosed)
	p.proxy.Call("close")
	p.funcs.Release()
	return nil
}

func (p *WebSocketProxy) closeWithError(err error) {
	if !p.closed {
		p.closed = true
		p.err = err
		close(p.q)
	}
}

func (p *WebSocketProxy) onData(this js.Value, args []js.Value) interface{} {
	iotime.Store(int64(args[2].Float()))
	n := args[1].Int()
	b := webSocketBuffers.Get().([]byte)

	if n > cap(b) {
		b = make([]byte, n)
	}
	b = b[:n]

	js.CopyBytesToGo(b, args[0])
	p.q <- b
	return nil
}

func (p *WebSocketProxy) onClose(this js.Value, args []js.Value) interface{} {
	p.closeWithError(ErrClosed)
	return nil
}

func (p *WebSocketProxy) onError(this js.Value, args []js.Value) interface{} {
	p.closeWithError(errors.New(args[0].String()))
	return nil
}
