package lhls

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

// NewIngress ...
func NewIngress(ctx context.Context, host *encoding.Host) (c *Ingress) {
	c = &Ingress{
		ctx:   ctx,
		host:  host,
		close: make(chan struct{}, 1),
	}

	c.server.HandlePublish = c.handlePublish

	return c
}

// Ingress ...
type Ingress struct {
	ctx        context.Context
	close      chan struct{}
	host       *encoding.Host
	server     rtmp.Server
	swarms     sync.Map
	swarmChans sync.Map
}

// Notify ...
func (h *Ingress) Notify(ch chan *encoding.Swarm) {
	go h.swarms.Range(func(_ interface{}, si interface{}) bool {
		ch <- si.(*encoding.Swarm)
		return true
	})

	h.swarmChans.Store(ch, ch)
}

// Stop ...
func (h *Ingress) Stop(ch chan *encoding.Swarm) {
	h.swarmChans.Delete(ch)
}

func (h *Ingress) handlePublish(conn *rtmp.Conn) {
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

	h.swarmChans.Range(func(_ interface{}, chi interface{}) bool {
		chi.(chan *encoding.Swarm) <- s
		return true
	})

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
		// h.host.RemoveSwarm(s.ID)
	}()
}

// ListenAndServe ...
func (h *Ingress) ListenAndServe() error {
	return h.server.ListenAndServe()
}

func (h *Ingress) copyPackets(w *chunkstream.Writer, src *rtmp.Conn) (err error) {
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
