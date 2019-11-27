package main

import (
	"context"
	"log"
	"runtime"
	"syscall/js"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/gobridge"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
)

func init() {
	// things fall apart after a few thousand allocations unless we run gc
	// manually. if this isn't a go bug we should think of a better way to
	// schedule this...
	go func() {
		for range time.NewTicker(5 * time.Second).C {
			runtime.GC()
		}
	}()
}

func consoleLog(args ...interface{}) {
	js.Global().Get("console").Call("log", args...)
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	gobridge.RegisterCallback("init", func(this js.Value, args []js.Value) (interface{}, error) {
		ctx := context.Background()

		h := encoding.NewHost(&encoding.HostOptions{
			Context: ctx,
			Transports: []encoding.Transport{
				&encoding.WRTCTransport{
					Adapter: encoding.NewJSWRTCAdapter(args[0]),
				},
			},
		})

		b := wasmio.NewBus(args[1])
		svc := service.New(h)
		go service.NewRPCHost(b, b, svc).Run(ctx)

		return nil, nil
	})

	select {}
}
