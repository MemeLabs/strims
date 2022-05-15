// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"io"
	"sync"

	"github.com/MemeLabs/strims/pkg/ioutil"
)

const connMTU = 16 * 1024

// Conn ...
type Conn interface {
	ioutil.BufferedWriteFlusher
	io.ReadCloser
	Buffered() int
	MTU() int
	SetQOSWeight(w uint64)
}

// NewConnPair ...
func NewConnPair() (Conn, Conn) {
	a, b := newConnPair()
	return &a, &b
}

func newConnPair() (conn, conn) {
	ar, aw := NewBufPipe()
	br, bw := NewBufPipe()

	aw.Grow(connMTU)
	bw.Grow(connMTU)

	return conn{ar, bw}, conn{br, aw}
}

// Conn ...
type conn struct {
	r *BufPipeReader
	w *BufPipeWriter
}

func (c *conn) Available() int {
	return connMTU - c.Buffered()
}

func (c *conn) AvailableBuffer() []byte {
	return c.w.AvailableBuffer()
}

// Write ...
func (c *conn) Write(p []byte) (int, error) {
	return c.w.Write(p)
}

// Flush ...
func (c *conn) Flush() error {
	defer c.w.Grow(connMTU)
	return c.w.Flush()
}

// Buffered ...
func (c *conn) Buffered() int {
	return c.w.Buffered()
}

// Close ...
func (c *conn) Close() error {
	c.w.Close()
	c.r.Close()
	return nil
}

// MTU ...
func (c *conn) MTU() int {
	return connMTU
}

func (c *conn) SetQOSWeight(w uint64) {}

// Read ...
func (c *conn) Read(p []byte) (int, error) {
	return c.r.Read(p)
}

// NewUnbufferedConnPair ...
func NewUnbufferedConnPair() (Conn, Conn) {
	a, b := newConnPair()
	return &unbufferedConn{conn: a}, &unbufferedConn{conn: b}
}

type unbufferedConn struct {
	mu sync.Mutex
	conn
}

func (c *unbufferedConn) Read(p []byte) (int, error) {
	n, err := c.conn.Read(p)
	// fmt.Printf("read %p %s", c, spew.Sdump(p[:n]))
	return n, err
}

func (c *unbufferedConn) Write(p []byte) (int, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	// fmt.Printf("write %p %s", c, spew.Sdump(p))
	n, err := c.conn.Write(p)
	if err != nil {
		return 0, err
	}
	if err := c.conn.Flush(); err != nil {
		return 0, err
	}
	return n, nil
}
