//go:build js
// +build js

package wasmio

import (
	"syscall/js"
)

// NewWorkerProxy ...
func NewWorkerProxy(bridge js.Value, service string) *Bus {
	ch := make(chan *Bus, 1)
	openBus := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		b, proxy := newBusFromProxy(args[0].Int())
		ch <- b
		return proxy
	})
	defer openBus.Release()

	proxy := jsObject.New()
	proxy.Set("openBus", openBus)
	bridge.Call("openWorker", service, proxy)

	return <-ch
}
