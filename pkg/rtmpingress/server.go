// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package rtmpingress

import (
	"context"
	"fmt"
	"net"
	"sync"
	"time"

	"github.com/nareix/joy5/format/rtmp"
	"go.uber.org/multierr"
	"go.uber.org/zap"
)

var rtmpPathPattern = "/live/%s"
var rtmpURIPattern = "rtmp://%s/live/%s"

// StreamAddr ...
type StreamAddr struct {
	URI string
	Key string
}

// Server ...
type Server struct {
	Addr         string
	CheckOrigin  func(a *StreamAddr, c *Conn) bool
	HandleStream func(a *StreamAddr, c *Conn)
	BaseContext  func(nc net.Conn) context.Context
	Logger       *zap.Logger

	streams  streams
	listener net.Listener
	conns    sync.Map
}

func (s *Server) logEvent(c *rtmp.Conn, nc net.Conn, e int) {
	if s.Logger != nil {
		s.Logger.Debug(
			"rtmp event",
			zap.Stringer("localAddr", nc.LocalAddr()),
			zap.Stringer("remoteAddr", nc.RemoteAddr()),
			zap.String("event", rtmp.EventString[e]),
		)
	}
}

func (s *Server) handleConn(c *rtmp.Conn, nc net.Conn) {
	var k string
	if _, err := fmt.Sscanf(c.URL.Path, rtmpPathPattern, &k); err != nil {
		return
	}

	ic := NewConn(s.connContext(nc), nc)
	defer ic.Close()

	a := &StreamAddr{
		Key: k,
		URI: fmt.Sprintf(rtmpURIPattern, s.Addr, k),
	}

	if s.CheckOrigin != nil && !s.CheckOrigin(a, ic) {
		return
	}

	stream, remove := s.streams.add(c.URL.Path)
	defer remove()

	if c.Publishing {
		go s.HandleStream(a, ic)

		stream.setPub(c)
	} else {
		stream.addSub(c.CloseNotify(), c)
	}
}

func (s *Server) connContext(nc net.Conn) context.Context {
	if s.BaseContext != nil {
		return s.BaseContext(nc)
	}
	return context.Background()
}

// Close ...
func (s *Server) Close() error {
	var errs []error

	if err := s.listener.Close(); err != nil {
		errs = append(errs, err)
	}

	s.conns.Range(func(key, _ any) bool {
		if err := key.(net.Conn).Close(); err != nil {
			errs = append(errs, err)
		}
		return true
	})

	if errs != nil {
		return multierr.Combine(errs...)
	}
	return nil
}

// Listen ...
func (s *Server) Listen() (err error) {
	s.listener, err = net.Listen("tcp", s.Addr)
	if err != nil {
		return err
	}
	defer func() { s.listener = nil }()

	srv := &rtmp.Server{
		LogEvent:         s.logEvent,
		HandleConn:       s.handleConn,
		HandshakeTimeout: time.Second * 10,
	}

	for {
		nc, err := s.listener.Accept()
		if err != nil {
			return err
		}

		s.conns.Store(nc, struct{}{})
		go func() {
			srv.HandleNetConn(nc)
			s.conns.Delete(nc)
		}()
	}
}

// NewConn ...
func NewConn(ctx context.Context, nc net.Conn) *Conn {
	ctx, cancel := context.WithCancel(ctx)
	return &Conn{
		Conn:   nc,
		ctx:    ctx,
		cancel: cancel,
	}
}

// Conn ...
type Conn struct {
	net.Conn
	ctx       context.Context
	cancel    context.CancelFunc
	closeOnce sync.Once
}

// Close ...
func (c *Conn) Close() (err error) {
	c.closeOnce.Do(func() {
		err = c.Conn.Close()
		c.cancel()
	})
	return
}

// CloseNotify ...
func (c *Conn) CloseNotify() <-chan struct{} {
	return c.ctx.Done()
}

// Context ...
func (c *Conn) Context() context.Context {
	return c.ctx
}
