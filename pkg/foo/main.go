package main

import (
	"context"
	"flag"
	"log"
	_ "net/http/pprof"
	"os"
	"os/signal"
	"syscall"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

type config struct {
	addr *string
	test *string
}

func (c *config) InitFlags() {
	c.addr = flag.String("addr", ":7881", "listen address")
	c.test = flag.String("test", "", "test remote addr")
	flag.Parse()
}

func (c *config) HostOptions() *encoding.HostOptions {
	return &encoding.HostOptions{
		Context: context.Background(),
		Transports: []encoding.Transport{
			&encoding.UDPTransport{
				Address: *c.addr,
			},
		},
	}
}

func main() {
	var cfg config
	cfg.InitFlags()

	log.Println("starting...")

	h := encoding.NewHost(cfg.HostOptions())
	go func() {
		if err := h.Run(); err != nil {
			log.Println(err)
			os.Exit(1)
		}
	}()

	if *cfg.test != "" {
		log.Fatal(h.TestSend(*cfg.test))
	} else {
		log.Fatal(h.TestReceive())
	}

	ch := make(chan os.Signal, 1)
	signal.Notify(ch)
	sig := <-ch
	log.Println("received signal:", sig)

	switch sig {
	case syscall.SIGINT:
		h.Shutdown()
	case syscall.SIGTERM:
		os.Exit(int(sig.(syscall.Signal)))
	}

	log.Println("goodbye")
}
