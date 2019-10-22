package main

import (
	"context"
	"crypto/rand"
	"encoding/binary"
	"fmt"
	"log"
	mathrand "math/rand"
	"net/http"
	_ "net/http/pprof"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

func init() {
	b := make([]byte, 8)
	_, err := rand.Read(b)
	if err != nil {
		panic(err)
	}
	mathrand.Seed(int64(binary.BigEndian.Uint64(b)))
}

func randAddr() string {
	return fmt.Sprintf(":%d", mathrand.Intn(50000)+10000)
}

func runA(ctx context.Context, ch chan joinOpts) {
	listenAddr := randAddr()

	h := encoding.NewHost(&encoding.HostOptions{
		Context: ctx,
		Transports: []encoding.Transport{
			&encoding.UDPTransport{
				Address: listenAddr,
			},
		},
	})

	go func() {
		srv := lhls.NewIngress(ctx, h)
		go srv.ListenAndServe()

		for s := range srv.DebugSwarms {
			ch <- joinOpts{
				SwarmID: s.ID,
				Address: "localhost" + listenAddr,
			}
		}
	}()

	if err := h.Run(); err != nil {
		log.Println(err)
	}
}

type joinOpts struct {
	SwarmID *encoding.SwarmID
	Address string
}

func runB(ctx context.Context, ch chan joinOpts) {
	h := encoding.NewHost(&encoding.HostOptions{
		Context: ctx,
		Transports: []encoding.Transport{
			&encoding.UDPTransport{
				Address: randAddr(),
			},
		},
	})

	srv := lhls.NewEgress()
	go srv.ListenAndServe()

	go func() {
		for opt := range ch {
			log.Printf("joining swarm %s at %s", opt.SwarmID, opt.Address)
			cr, err := h.JoinSwarm(opt.SwarmID, opt.Address)
			if err != nil {
				log.Println(err)
			}

			// TODO: move this to lhls.Egress
			c := &lhls.Channel{
				Stream: lhls.NewDefaultStream(),
			}
			srv.AddChannel(c)

			// TODO: move this to lhls.Channel
			go func() {
				defer srv.RemoveChannel(c)

				r, err := chunkstream.NewReader(cr, int64(cr.Offset()))
				if err != nil {
					log.Panic(err)
				}

				b := make([]byte, 4096)
				for {
					w := c.Stream.NextWriter()
					var wn int
					for {
						n, err := r.Read(b)
						if err != nil && err != chunkstream.EOR {
							log.Println("read failed with error", err)
						}
						w.Write(b[:n])
						wn += n

						if err == chunkstream.EOR {
							break
						}
						if ctx.Err() == context.Canceled {
							return
						}
					}

					debug.Green("closed chunk", wn)
					if err := w.Close(); err != nil {
						log.Println("error closing segment", err)
						return
					}
				}
			}()
		}
	}()

	if err := h.Run(); err != nil {
		log.Println(err)
	}
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)
	go func() {
		log.Println(http.ListenAndServe("localhost:6060", nil))
	}()

	log.Println("starting...")

	ctx, cancel := context.WithCancel(context.Background())
	ch := make(chan joinOpts)
	wg := sync.WaitGroup{}
	wg.Add(2)
	go func() {
		runA(ctx, ch)
		wg.Done()
	}()
	go func() {
		runB(ctx, ch)
		wg.Done()
	}()

	signals := make(chan os.Signal, 1)
	signal.Notify(signals)
	sig := <-signals
	log.Println("received signal:", sig)

	switch sig {
	case syscall.SIGINT:
		cancel()
	case syscall.SIGTERM:
		os.Exit(int(sig.(syscall.Signal)))
	}

	wg.Wait()
	log.Println("goodbye")
}
