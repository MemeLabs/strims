package main

import (
	"context"
	"crypto/rand"
	"encoding/binary"
	"fmt"
	"io"
	"log"
	mathrand "math/rand"
	"net/http"
	_ "net/http/pprof"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

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
	return fmt.Sprintf("127.0.0.1:%d", mathrand.Intn(50000)+10000)
}

func runA(ctx context.Context, ch chan joinOpts) {
	listenAddr := randAddr()

	h := encoding.NewHost(&encoding.HostOptions{
		Context: ctx,
		Transports: []encoding.Transport{
			&encoding.UDPTransport{
				Address: listenAddr,
			},
			&encoding.WRTCTransport{
				Adapter: &encoding.NativeWRTCAdapter{
					SignalAddress: "0.0.0.0:8082",
				},
			},
		},
	})

	go func() {
		srv := lhls.NewIngress(ctx, h)
		go srv.ListenAndServe()

		swarms := make(chan *encoding.Swarm)
		srv.Notify(swarms)
		defer srv.Stop(swarms)

		for s := range swarms {
			ch <- joinOpts{
				SwarmID: s.ID,
				Address: listenAddr,
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

	srv := lhls.NewEgress(lhls.DefaultEgressOptions)
	go srv.ListenAndServe()

	go func() {
		for opt := range ch {
			log.Printf("joining swarm %s at %s", opt.SwarmID, opt.Address)
			uri := encoding.TransportURI(encoding.UDPScheme + opt.Address)
			cr, err := h.JoinSwarm(opt.SwarmID, uri)
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
						if err != nil && err != io.EOF {
							log.Println("read failed with error", err)
						}
						w.Write(b[:n])
						wn += n

						if err == io.EOF {
							break
						}
						if ctx.Err() == context.Canceled {
							return
						}
					}

					// debug.Green("closed chunk", wn)
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
		log.Println(http.ListenAndServe("127.0.0.1:6060", nil))
	}()

	log.Println("starting...")

	ctx, cancel := context.WithCancel(context.Background())
	joinSrc := make(chan joinOpts)
	joinDsts := make([]chan joinOpts, 0)
	wg := sync.WaitGroup{}

	for i := 0; i < 1; i++ {
		wg.Add(1)
		go func() {
			joinDst := make(chan joinOpts)
			joinDsts = append(joinDsts, joinDst)
			runB(ctx, joinDst)
			wg.Done()
		}()
	}

	go func() {
		for join := range joinSrc {
			debug.Blue(join.SwarmID.String(), join.Address)
			for _, dst := range joinDsts {
				dst <- join
			}
		}
	}()

	wg.Add(1)
	go func() {
		time.Sleep(100 * time.Millisecond)
		runA(ctx, joinSrc)
		wg.Done()
	}()

	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGINT, syscall.SIGTERM)
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
