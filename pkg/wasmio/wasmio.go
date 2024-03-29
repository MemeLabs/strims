// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package wasmio

import (
	"errors"
	"syscall/js"

	"github.com/MemeLabs/strims/pkg/timeutil"
)

var jsObject = js.Global().Get("Object")
var jsUint8Array = js.Global().Get("Uint8Array")

// ErrClosed is the error used for read or write operations on a closed proxy.
var ErrClosed = errors.New("read/write on closed proxy")

// ErrOperationTimeout ...
var ErrOperationTimeout = errors.New("operation timed out")

func syncTime(t int) {
	timeutil.SyncNow(int64(t * 1000000))
}

//go:noescape
func channelWrite(cid int, src []byte) (int, bool)

//go:noescape
func channelRead(cid int, dst []byte) (int, bool)

//go:noescape
func channelClose(cid int) bool

func interfacesFromStrings(ss []string) []any {
	ifs := make([]any, len(ss))
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
