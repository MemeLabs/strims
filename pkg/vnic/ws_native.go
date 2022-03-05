//go:build !js

package vnic

import (
	"crypto/tls"
	"fmt"
	"net/http"

	"github.com/MemeLabs/go-ppspp/pkg/httputil"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

type WSInterfaceOptions struct {
	ServeMux *httputil.MapServeMux
}

// NewWSInterface ...
func NewWSInterface(logger *zap.Logger, options WSInterfaceOptions) Interface {
	return &wsInterface{
		logger:  logger,
		options: options,
	}
}

type wsInterface struct {
	logger  *zap.Logger
	options WSInterfaceOptions
	path    string
}

func (f *wsInterface) ValidScheme(scheme string) bool {
	return scheme == "ws" || scheme == "wss"
}

func (f *wsInterface) Listen(h *Host) error {
	if f.options.ServeMux != nil {
		f.path = fmt.Sprintf("/%x", h.profileKey.Public)
		f.logger.Debug("ws vnic listener starting", zap.String("path", f.path))
		f.options.ServeMux.HandleWSFunc(f.path, func(c *websocket.Conn) {
			h.AddLink(httputil.NewWSReadWriter(c))
		})
	}
	return nil
}

func (f *wsInterface) Close() error {
	if f.options.ServeMux != nil {
		f.options.ServeMux.StopHandling(f.path)
	}
	return nil
}

func (f *wsInterface) Dial(addr InterfaceAddr) (Link, error) {
	d := &websocket.Dialer{
		TLSClientConfig: &tls.Config{
			InsecureSkipVerify: addr.(WebSocketAddr).InsecureSkipVerifyTLS,
		},
	}
	c, _, err := d.Dial(addr.(WebSocketAddr).URL, http.Header{})
	if err != nil {
		return nil, err
	}
	return httputil.NewWSReadWriter(c), nil
}
