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

func startTestWriter(ctx context.Context, w io.Writer) (err error) {
	cw, err := chunkstream.NewWriter(w)
	if err != nil {
		return
	}
	t := time.NewTicker(10 * time.Millisecond)

	b := make([]byte, 9e3)
	for i := range b {
		b[i] = 255
	}
	// _, err = rand.Read(b)
	// if err != nil {
	// 	return
	// }
	sum := 0

	for {
		select {
		case <-t.C:
			n, err := cw.Write(b)
			if err != nil {
				return err
			}
			sum += n
			// if sum > 9e4 {
			if sum > 3e6 {
				if err = cw.Flush(); err != nil {
					return err
				}
				sum = 0
			}
		case <-ctx.Done():
			return ctx.Err()
		}
	}
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
		},
	})

	go func() {
		time.Sleep(200 * time.Millisecond)

		w, err := encoding.NewWriter(encoding.DefaultSwarmWriterOptions)
		if err != nil {
			panic(err)
		}

		s := w.Swarm()
		ch <- joinOpts{
			SwarmID: s.ID,
			Address: listenAddr,
		}

		h.HostSwarm(s)
		err = startTestWriter(ctx, w)
		if err != nil {
			log.Println(err)
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

	go func() {
		for opt := range ch {
			log.Printf("joining swarm %s at %s", opt.SwarmID, opt.Address)
			uri := encoding.TransportURI(encoding.UDPScheme + opt.Address)
			cr, err := h.JoinSwarm(opt.SwarmID, uri)
			if err != nil {
				log.Panic(err)
			}

			go func() {
				log.Println("offset", int64(cr.Offset()))
				r, err := chunkstream.NewReader(cr, int64(cr.Offset()))
				if err != nil {
					log.Panic(err)
				}
				b := make([]byte, 4*1024)
				total := 0
				for {
					n, err := r.Read(b)
					if err == io.EOF {
						debug.Green("got chunk", total)
						total = 0
					} else if err != nil {
						log.Println("read failed with error", err)
						break
					}
					total += n

					if ctx.Err() == context.Canceled {
						break
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
