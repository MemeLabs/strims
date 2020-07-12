package rtmpingress

import (
	"fmt"
	"net"
	"time"

	"github.com/nareix/joy5/format/rtmp"
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
	CheckOrigin  func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) bool
	HandleStream func(a *StreamAddr, c *rtmp.Conn, nc net.Conn)
	Logger       *zap.Logger

	streams  streams
	listener net.Listener
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

	a := &StreamAddr{
		Key: k,
		URI: fmt.Sprintf(rtmpURIPattern, s.Addr, k),
	}

	if s.CheckOrigin != nil && !s.CheckOrigin(a, c, nc) {
		nc.Close()
		return
	}

	stream, remove := s.streams.add(c.URL.Path)
	defer remove()

	if c.Publishing {
		go s.HandleStream(a, c, nc)

		stream.setPub(c)
	} else {
		stream.addSub(c.CloseNotify(), c)
	}
}

// Close ...
func (s *Server) Close() error {
	return s.listener.Close()
}

// Listen ...
func (s *Server) Listen() error {
	l, err := net.Listen("tcp", s.Addr)
	if err != nil {
		return err
	}
	s.listener = l

	srv := &rtmp.Server{
		LogEvent:         s.logEvent,
		HandleConn:       s.handleConn,
		HandshakeTimeout: time.Second * 10,
	}

	for {
		nc, err := l.Accept()
		if err != nil {
			time.Sleep(time.Second)
			continue
		}
		go srv.HandleNetConn(nc)
	}
}
