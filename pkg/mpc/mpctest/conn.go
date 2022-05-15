// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpctest

import (
	"bufio"
	"net"
)

// Pipe ...
func Pipe() (ca, cb *Conn) {
	a, b := net.Pipe()
	return NewConn(a), NewConn(b)
}

// NewConn ...
func NewConn(c net.Conn) *Conn {
	return &Conn{
		ReadWriter: bufio.NewReadWriter(bufio.NewReader(c), bufio.NewWriter(c)),
	}
}

// Conn ...
type Conn struct {
	*bufio.ReadWriter
	r, w, f int
}

// ReadBytes ...
func (c *Conn) ReadBytes() int {
	return c.r
}

// WrittenBytes ...
func (c *Conn) WrittenBytes() int {
	return c.w
}

// Flushes ...
func (c *Conn) Flushes() int {
	return c.f
}

// Write ...
func (c *Conn) Write(b []byte) (n int, err error) {
	n, err = c.ReadWriter.Write(b)
	c.w += n
	return
}

// Read ...
func (c *Conn) Read(b []byte) (n int, err error) {
	n, err = c.ReadWriter.Read(b)
	c.r += n
	return
}

// Flush ...
func (c *Conn) Flush() error {
	c.f++
	return c.ReadWriter.Flush()
}
