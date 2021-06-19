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
	p.id = bridge.Call(method, append(args, proxy)...).Int()

	<-p.q

	return p, p.err
}

func newChannelFromProxy(mtu int, id int) (*channel, js.Value) {
	p := &channel{
		mtu: mtu,
		id:  id,
		q:   make(chan *[]byte, 16),
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
	id     int
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

	t, ok := channelWrite(p.id, b)
	if !ok {
		return 0, ErrClosed
	}

	syncTime(t)
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
	channelClose(p.id)
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
	b := pool.Get(args[0].Int())
	t, ok := channelRead(p.id, *b)
	if !ok {
		p.closeWithError(ErrClosed)
		return nil
	}

	syncTime(t)
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
