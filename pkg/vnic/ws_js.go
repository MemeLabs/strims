//go:build js

package vnic

import (
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
)

// NewWSInterface ...
func NewWSInterface(logger *zap.Logger, bridge js.Value) Interface {
	return &wsInterface{
		logger: logger,
		bridge: bridge,
	}
}

// wsInterface ...
type wsInterface struct {
	logger *zap.Logger
	bridge js.Value
}

// ValidScheme ...
func (d *wsInterface) ValidScheme(scheme string) bool {
	return scheme == "ws" || scheme == "wss"
}

// Dial ...
func (d *wsInterface) Dial(addr InterfaceAddr) (Link, error) {
	url := addr.(WebSocketAddr).String()
	return wasmio.NewWebSocketProxy(d.bridge, url)
}
