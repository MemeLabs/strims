// +build js,wasm

package encoding

import (
	"context"
	"errors"
	"sync"
	"syscall/js"
	"time"
)

// jsWRTCConn ...
type jsWRTCConn struct {
	id int
	a  *JSWRTCAdapter
}

func (j jsWRTCConn) isWRTCConn() {}

// NewJSWRTCAdapter ...
func NewJSWRTCAdapter(v js.Value) (r *JSWRTCAdapter) {
	r = &JSWRTCAdapter{
		v:    v,
		cond: sync.Cond{L: &sync.Mutex{}},
		b:    v.Get("data"),
	}
	v.Set("ondata", js.FuncOf(r.handleData))
	v.Set("onclose", js.FuncOf(r.handleClose))
	return
}

// JSWRTCAdapter ...
type JSWRTCAdapter struct {
	transportState
	v    js.Value
	cond sync.Cond
	cid  int
	n    int
	b    js.Value
}

func (a *JSWRTCAdapter) listen(ctx context.Context) error {
	return nil
}

func (a *JSWRTCAdapter) close() error {
	return nil
}

func (a *JSWRTCAdapter) handleData(this js.Value, args []js.Value) interface{} {
	a.cond.L.Lock()
	defer a.cond.L.Unlock()

	a.cid = args[0].Int()
	a.n = args[1].Int()
	a.cond.Signal()

	return nil
}

func (a *JSWRTCAdapter) handleClose(this js.Value, args []js.Value) interface{} {
	// is there cleanup to do here...?

	return nil
}

func (a *JSWRTCAdapter) dial(uri TransportURI) (tc wrtcConn, err error) {
	t, err := resolveThing(uri)
	if err != nil {
		return
	}

	tcc := make(chan jsWRTCConn)

	cb := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		tcc <- jsWRTCConn{
			id: args[0].Int(),
		}

		return nil
	})
	defer cb.Release()

	o := js.ValueOf(map[string]interface{}{
		"signalProtocol": js.ValueOf(t.sp),
		"signalAddress":  js.ValueOf(t.sa),
		"id":             js.ValueOf(t.id),
	})
	a.v.Call("dial", o, cb)

	select {
	case tc = <-tcc:
	case <-time.After(30 * time.Second):
		err = errors.New("timeout")
	}

	return
}

func (a *JSWRTCAdapter) write(b []byte, c wrtcConn) (err error) {
	n := js.CopyBytesToJS(a.b, b)
	// log.Println("write", spew.Sdump(b[:n]))
	a.v.Call("write", js.ValueOf(c.(jsWRTCConn).id), js.ValueOf(n))
	return nil
}

func (a *JSWRTCAdapter) read(b []byte) (n int, c wrtcConn, err error) {
	a.cond.L.Lock()
	defer a.cond.L.Unlock()
	a.cond.Wait()

	n = js.CopyBytesToGo(b[:a.n], a.b)
	// log.Println("read", spew.Sdump(b[:a.n]))
	c = jsWRTCConn{
		id: a.cid,
		a:  a,
	}

	return
}

func (a *JSWRTCAdapter) closeConn(c wrtcConn) (err error) {
	a.v.Call("close", js.ValueOf(c.(jsWRTCConn).id))
	return nil
}
