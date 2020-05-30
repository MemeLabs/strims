// +build js

package wasmio

import (
	"errors"
	"syscall/js"
)

var jsObject = js.Global().Get("Object")
var jsUint8Array = js.Global().Get("Uint8Array")

// ErrClosed is the error used for read or write operations on a closed proxy.
var ErrClosed = errors.New("read/write on closed proxy")

// ErrOperationTimeout ...
var ErrOperationTimeout = errors.New("operation timed out")

func interfacesFromStrings(ss []string) []interface{} {
	ifs := make([]interface{}, len(ss))
	for i, s := range ss {
		ifs[i] = s
	}
	return ifs
}

// Funcs ...
type Funcs []js.Func

// Register ...
func (f *Funcs) Register(fn js.Func) js.Func {
	*f = append(*f, fn)
	return fn
}

// Release ...
func (f *Funcs) Release() {
	for _, fn := range *f {
		fn.Release()
	}
}
