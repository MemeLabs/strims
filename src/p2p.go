package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"runtime"
	"syscall/js"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/gobridge"
)

func init() {
	// things fall apart after a few thousand allocations unless we run gc
	// manually. if this isn't a go bug we should think of a better way to
	// schedule this...
	go func() {
		for range time.NewTicker(5 * time.Second).C {
			// for range time.NewTicker(100 * time.Millisecond).C {
			runtime.GC()
		}
	}()
}

func consoleLog(args ...interface{}) {
	js.Global().Get("console").Call("log", args...)
}

// NewJSWriter ...
func NewJSWriter(v js.Value) JSWriter {
	return JSWriter{
		v: v,
		d: v.Get("data"),
	}
}

// JSWriter ...
type JSWriter struct {
	v js.Value
	d js.Value
}

// Write ...
func (j JSWriter) Write(p []byte) (n int, err error) {
	n = js.CopyBytesToJS(j.d, p)
	j.v.Call("ondata", js.ValueOf(n))
	return
}

// Flush ...
func (j JSWriter) Flush() error {
	j.v.Call("onflush")
	return nil
}

func copyChunksToJS(ctx context.Context, r *encoding.ChunkBufferReader, w JSWriter) (err error) {
	log.Println("starting reading from", r.Offset())
	cr, err := chunkstream.NewReader(r, int64(r.Offset()))
	if err != nil {
		log.Panic(err)
	}

	b := make([]byte, 4096)
	for {
		fmt.Println("discarding...")
		if _, err = cr.Read(b); err != nil {
			if err == io.EOF {
				log.Println("done discarding")
				break
			}
			return
		}
	}
	for {
		n, err := cr.Read(b)
		if err != nil && err != io.EOF {
			log.Println("read failed with error", err)
			return err
		}
		if n != 0 {
			w.Write(b[:n])
		}
		if err == io.EOF {
			fmt.Println("flush")
			w.Flush()
		}

		if ctx.Err() == context.Canceled {
			return ctx.Err()
		}
	}
}

type joinOpts struct {
	SwarmID *encoding.SwarmID
	Adapter *encoding.JSWRTCAdapter
	Address string
	Writer  JSWriter
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)
	opts := make(chan joinOpts)

	gobridge.RegisterCallback("init", func(this js.Value, args []js.Value) (interface{}, error) {
		sid, err := encoding.ParseSwarmID(args[0].String())
		if err != nil {
			return nil, err
		}

		opts <- joinOpts{
			SwarmID: sid,
			Adapter: encoding.NewJSWRTCAdapter(args[1]),
			Address: "wrtc://ws/192.168.0.111:8082/signal",
			Writer:  NewJSWriter(args[2]),
		}
		return nil, nil
	})

	go func() {
		opt := <-opts

		ctx := context.Background()

		h := encoding.NewHost(&encoding.HostOptions{
			Context: ctx,
			Transports: []encoding.Transport{
				&encoding.WRTCTransport{
					Adapter: opt.Adapter,
				},
			},
		})

		go func() {
			uri := encoding.TransportURI(opt.Address)
			cr, err := h.JoinSwarm(opt.SwarmID, uri)
			if err != nil {
				log.Println(err)
			}

			if err := copyChunksToJS(ctx, cr, opt.Writer); err != nil {
				log.Println("error copying chunk buffer to js", err)
			}
		}()

		if err := h.Run(); err != nil {
			log.Println(err)
		}
	}()

	select {}
}
