// +build js,wasm

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
func (d *wsInterface) Dial(h *Host, addr InterfaceAddr) error {
	url := addr.(WebSocketAddr).String()

	c, err := wasmio.NewWebSocketProxy(d.bridge, url)
	if err != nil {
		return err
	}

	h.AddLink(c)
	return nil
}
