package main

import (
	"context"
	"log"
	"os"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/service"
)

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	ctx := context.Background()

	h := encoding.NewHost(&encoding.HostOptions{
		Context: ctx,
		Transports: []encoding.Transport{
			&encoding.UDPTransport{
				Address: "0.0.0.0:31929",
			},
			&encoding.WRTCTransport{
				Adapter: &encoding.NativeWRTCAdapter{
					SignalAddress: "0.0.0.0:8082",
				},
			},
		},
	})

	svc := service.New(h)
	go service.NewRPCHost(os.Stdout, os.Stdin, svc).Run(ctx)

	select {}
}
