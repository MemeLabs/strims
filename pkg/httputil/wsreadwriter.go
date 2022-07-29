// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package httputil

import (
	"errors"
	"io"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/gorilla/websocket"
)

const wsMTU = 64 * 1024

type WSOptions struct {
	WriteTimeout time.Duration
	ReadTimeout  time.Duration
	PingInterval time.Duration
}

func (o *WSOptions) Assign(u WSOptions) {
	if u.WriteTimeout != 0 {
		o.WriteTimeout = u.WriteTimeout
	}
	if u.ReadTimeout != 0 {
		o.ReadTimeout = u.ReadTimeout
	}
	if u.PingInterval != 0 {
		o.PingInterval = u.PingInterval
	}
}

var DefaultWSOptions = WSOptions{
	WriteTimeout: 5 * time.Second,
	ReadTimeout:  25 * time.Second,
	PingInterval: 20 * time.Second,
}

// ErrUnexpectedMessageType ...
var ErrUnexpectedMessageType = errors.New("unexpected non-binary message type")

// NewDefaultWSReadWriter ...
func NewDefaultWSReadWriter(c *websocket.Conn) *WSReadWriter {
	return NewWSReadWriter(c, DefaultWSOptions)
}

func NewWSReadWriter(c *websocket.Conn, opt WSOptions) *WSReadWriter {
	o := DefaultWSOptions
	o.Assign(opt)

	w := &WSReadWriter{
		options: o,
		conn:    c,
	}

	w.conn.SetReadDeadline(timeutil.Now().Add(o.ReadTimeout).Time())
	w.conn.SetPongHandler(func(string) error {
		w.conn.SetReadDeadline(timeutil.Now().Add(o.ReadTimeout).Time())
		return nil
	})

	w.stopPing = timeutil.DefaultTickEmitter.Subscribe(o.PingInterval, func(t timeutil.Time) {
		w.writeLock.Lock()
		defer w.writeLock.Unlock()
		w.conn.WriteControl(websocket.PingMessage, nil, t.Add(o.WriteTimeout).Time())
	}, nil)

	return w
}

// WSReadWriter ...
type WSReadWriter struct {
	options   WSOptions
	conn      *websocket.Conn
	reader    io.Reader
	writeLock sync.Mutex
	stopPing  timeutil.StopFunc
}

// MTU ...
func (w *WSReadWriter) MTU() int {
	return wsMTU
}

// Read ...
func (w *WSReadWriter) Read(b []byte) (n int, err error) {
	if w.reader == nil {
		var t int
		t, w.reader, err = w.conn.NextReader()
		if err != nil {
			return
		}
		if t != websocket.BinaryMessage {
			return 0, ErrUnexpectedMessageType
		}
	}

	n, err = w.reader.Read(b)
	if err == io.EOF {
		w.reader = nil
		err = nil

		if n == 0 {
			return w.Read(b)
		}
	}

	return
}

// Write ...
func (w *WSReadWriter) Write(b []byte) (int, error) {
	w.writeLock.Lock()
	defer w.writeLock.Unlock()

	if err := w.conn.SetWriteDeadline(timeutil.Now().Add(w.options.WriteTimeout).Time()); err != nil {
		return 0, err
	}

	if err := w.conn.WriteMessage(websocket.BinaryMessage, b); err != nil {
		return 0, err
	}

	return len(b), nil
}

// Close ...
func (w *WSReadWriter) Close() error {
	w.stopPing()
	return w.conn.Close()
}
