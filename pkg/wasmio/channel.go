// +build js

package wasmio

import (
	"errors"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
)

// newChannel ...
func newChannel(mtu int, bridge js.Value, method string, args ...interface{}) (*channel, error) {
	p := &channel{
		mtu: mtu,
		q:   make(chan *[]byte, 16),
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
	p.proxy = bridge.Call(method, append(args, proxy)...)

	<-p.q

	return p, p.err
}

func newChannelFromProxy(mtu int, v js.Value) (*channel, js.Value) {
	p := &channel{
		mtu:   mtu,
		proxy: v,
		q:     make(chan *[]byte, 16),
	}

	proxy := jsObject.New()
	proxy.Set("ondata", p.funcs.Register(js.FuncOf(p.onData)))
	proxy.Set("onclose", p.funcs.Register(js.FuncOf(p.onClose)))
	proxy.Set("onerror", p.funcs.Register(js.FuncOf(p.onError)))

	return p, proxy
}

// channel ...
type channel struct {
	mtu    int
	proxy  js.Value
	funcs  Funcs
	q      chan *[]byte
	b      *[]byte
	off    int
	closed bool
	err    error
}

// MTU ...
func (p *channel) MTU() int {
	return p.mtu
}

// Write ...
func (p *channel) Write(b []byte) (int, error) {
	if p.closed {
		return 0, p.err
	}

	data := jsUint8Array.New(len(b))
	js.CopyBytesToJS(data, b)
	now := p.proxy.Call("write", data)
	syncTime(now.Int())
	return len(b), nil
}

// Read ...
func (p *channel) Read(b []byte) (n int, err error) {
	if p.b == nil || p.off == len(*p.b) {
		if p.b != nil {
			pool.Put(p.b)
		}

		b, ok := <-p.q
		if !ok {
			return 0, p.err
		}

		p.b = b
		p.off = 0
	}

	n = len(*p.b) - p.off
	if n > len(b) {
		n = len(b)
	}

	copy(b[:n], (*p.b)[p.off:])
	p.off += n
	return n, nil
}

// Close ...
func (p *channel) Close() error {
	p.closeWithError(ErrClosed)
	p.proxy.Call("close")
	p.funcs.Release()
	return nil
}

func (p *channel) closeWithError(err error) {
	if !p.closed {
		p.closed = true
		p.err = err
		close(p.q)
	}
}

func (p *channel) onData(this js.Value, args []js.Value) interface{} {
	syncTime(args[2].Int())
	b := pool.Get(args[1].Int())
	js.CopyBytesToGo(*b, args[0])
	p.q <- b
	return nil
}

func (p *channel) onClose(this js.Value, args []js.Value) interface{} {
	p.closeWithError(ErrClosed)
	return nil
}

func (p *channel) onError(this js.Value, args []js.Value) interface{} {
	p.closeWithError(errors.New(args[0].String()))
	return nil
}
