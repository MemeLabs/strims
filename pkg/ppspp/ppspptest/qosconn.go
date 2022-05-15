// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"errors"
	"sync"

	"github.com/MemeLabs/strims/pkg/vnic/qos"
)

var errQOSConnClosed = errors.New("qos conn closed")

// NewQOSConn ...
func NewQOSConn(c Conn, qs *qos.Session) *QOSConn {
	return &QOSConn{
		Conn: c,
		qs:   qs,
		qp: qosConnPacket{
			ch: make(chan struct{}, 1),
		},
		close: make(chan struct{}),
	}
}

// QOSConn ...
type QOSConn struct {
	Conn
	qs        *qos.Session
	qp        qosConnPacket
	close     chan struct{}
	closeOnce sync.Once
}

// Close ...
func (c *QOSConn) Close() error {
	c.closeOnce.Do(func() { close(c.close) })
	return c.Conn.Close()
}

// Flush ...
func (c *QOSConn) Flush() error {
	c.qp.size = uint64(c.Conn.Buffered())
	c.qs.Enqueue(&c.qp)
	select {
	case <-c.qp.ch:
	case <-c.close:
		return errQOSConnClosed
	}

	return c.Conn.Flush()
}

type qosConnPacket struct {
	size uint64
	ch   chan struct{}
}

func (p *qosConnPacket) Size() uint64 {
	return p.size
}

func (p *qosConnPacket) Send() {
	p.ch <- struct{}{}
}
