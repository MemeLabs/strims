// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package wasmio

import (
	"syscall/js"
)

const webSocketMTU = 64 * 1024

// WebSocketProxy ...
type WebSocketProxy interface {
	MTU() int
	Write(b []byte) (int, error)
	Read(b []byte) (n int, err error)
	Close() error
}

// NewWebSocketProxy ...
func NewWebSocketProxy(bridge js.Value, uri string) (WebSocketProxy, error) {
	return newChannel(webSocketMTU, bridge, "openWebSocket", uri)
}
