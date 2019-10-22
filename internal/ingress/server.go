package ingress

import (
	"context"
	"log"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/nareix/joy4/format"
	"github.com/nareix/joy4/format/rtmp"
	"github.com/nareix/joy4/format/ts"
)

func init() {
	format.RegisterAll()
}

// New ...
func New(ctx context.Context, host *encoding.Host) (c *Server) {
	c = &Server{
		ctx:         ctx,
		host:        host,
		close:       make(chan struct{}, 1),
		DebugSwarms: make(chan *encoding.Swarm, 0),
	}

	c.server.HandlePublish = c.handlePublish

	return c
}

// Server ...
type Server struct {
	ctx   context.Context
	close chan struct{}
	host  *encoding.Host

	server rtmp.Server
	swarms sync.Map

	DebugSwarms chan *encoding.Swarm
}

func (h *Server) handlePublish(conn *rtmp.Conn) {
	// app, stream := rtmp.SplitPath(conn.URL)
	w, err := encoding.NewWriter(encoding.DefaultSwarmWriterOptions)
	if err != nil {
		log.Println("error creating writer", err)
		return
	}
	cw, err := chunkstream.NewWriter(w)
	if err != nil {
		return
	}

	s := w.Swarm()
	h.swarms.Store(s.ID.String(), s)
	h.host.HostSwarm(s)

	select {
	case h.DebugSwarms <- s:
	default:
		log.Println("unable to publish swarm, discarding")
		return
	}

	go func() {
		if err := h.copyPackets(cw, conn); err != nil {
			log.Printf("SwarmWriter closed %v", err)
		}

		if err := cw.Flush(); err != nil {
			log.Println("error flushing output", err)
		}
		if err := w.Flush(); err != nil {
			log.Println("error flushing output", err)
		}

		if err := w.Close(); err != nil {
			log.Println("error closing", err)
		}

		h.swarms.Delete(s.ID.String())
		h.host.RemoveSwarm(s.ID)
	}()
}

// ListenAndServe ...
func (h *Server) ListenAndServe() error {
	return h.server.ListenAndServe()
}

func (h *Server) copyPackets(w *chunkstream.Writer, src *rtmp.Conn) (err error) {
	streams, err := src.Streams()
	if err != nil {
		return
	}
	pkt, err := src.ReadPacket()
	if err != nil {
		return
	}

	for {
		muxer := ts.NewMuxer(w)
		if err = muxer.WriteHeader(streams); err != nil {
			return
		}

		for {
			if err = muxer.WritePacket(pkt); err != nil {
				return
			}

			if pkt, err = src.ReadPacket(); err != nil {
				return
			}
			if pkt.IsKeyFrame {
				break
			}
		}

		if err = muxer.WriteTrailer(); err != nil {
			return
		}
		if err = w.Flush(); err != nil {
			return
		}
	}
}
