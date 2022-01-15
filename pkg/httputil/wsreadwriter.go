package httputil

import (
	"errors"
	"io"
	"sync"

	"github.com/gorilla/websocket"
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
