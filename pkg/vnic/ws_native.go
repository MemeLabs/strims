//go:build !js

package vnic

import (
	"crypto/tls"
	"errors"
	"io"
	"net/http"
	"sync"

	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

var wsMTU = 64 * 1024

// ErrUnexpectedMessageType ...
var ErrUnexpectedMessageType = errors.New("unexpected non-binary message type")

// NewWSReadWriter ...
func NewWSReadWriter(c *websocket.Conn) *WSReadWriter {
	return &WSReadWriter{
		c: c,
	}
}

// WSReadWriter ...
type WSReadWriter struct {
	c *websocket.Conn
	l sync.Mutex
	r io.Reader
}

// MTU ...
func (w *WSReadWriter) MTU() int {
	return wsMTU
}

// Read ...
func (w *WSReadWriter) Read(b []byte) (n int, err error) {
	if w.r == nil {
		var t int
		t, w.r, err = w.c.NextReader()
		if err != nil {
			return
		}
		if t != websocket.BinaryMessage {
			return 0, ErrUnexpectedMessageType
		}
	}

	n, err = w.r.Read(b)
	if err == io.EOF {
		w.r = nil
		err = nil

		if n == 0 {
			return w.Read(b)
		}
	}

	return
}

// Write ...
func (w *WSReadWriter) Write(b []byte) (int, error) {
	w.l.Lock()
	defer w.l.Unlock()
	err := w.c.WriteMessage(websocket.BinaryMessage, b)
	return len(b), err
}

// Close ...
func (w *WSReadWriter) Close() error {
	return w.c.Close()
}

// NewWSInterface ...
func NewWSInterface(logger *zap.Logger, serverAddress string) Interface {
	return &wsInterface{
		logger:        logger,
		ServerAddress: serverAddress,
	}
}

type wsInterface struct {
	logger        *zap.Logger
	ServerAddress string
	serverLock    sync.Mutex
	server        *http.Server
}

func (f *wsInterface) ValidScheme(scheme string) bool {
	return scheme == "ws" || scheme == "wss"
}

func (f *wsInterface) Listen(h *Host) error {
	if f.ServerAddress == "" {
		return nil
	}

	upgrader := websocket.Upgrader{
		CheckOrigin: func(r *http.Request) bool {
			return true
		},
	}

	f.serverLock.Lock()
	if f.server != nil {
		f.serverLock.Unlock()
		return errors.New("server already running")
	}

	srv := &http.Server{
		Addr: f.ServerAddress,
		Handler: http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			c, err := upgrader.Upgrade(w, r, nil)
			if err != nil {
				f.logger.Warn("websocket upgrade error", zap.Error(err))
				return
			}
			h.AddLink(NewWSReadWriter(c))
		}),
	}

	f.server = srv
	f.serverLock.Unlock()

	f.logger.Debug("starting websocket server", zap.String("address", f.ServerAddress))
	return srv.ListenAndServe()
}

func (f *wsInterface) Close() error {
	f.serverLock.Lock()
	defer f.serverLock.Unlock()

	if f.server == nil {
		return errors.New("server is not running")
	}

	err := f.server.Close()
	f.server = nil
	return err
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
	return NewWSReadWriter(c), nil
}
