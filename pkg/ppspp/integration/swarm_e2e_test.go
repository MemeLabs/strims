package integration

import (
	"context"
	"io"
	"io/ioutil"
	"log"
	"math/rand"
	"net/http"
	_ "net/http/pprof"
	"sync/atomic"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/stretchr/testify/assert"
)

func TestSwarmE2E(t *testing.T) {
	key := ppspptest.Key()
	id := ppspp.NewSwarmID(key.Public)
	options := ppspp.SwarmOptions{
		LiveWindow: 1 << 12,
	}

	type client struct {
		id     []byte
		swarm  *ppspp.Swarm
		runner *ppspp.Runner
	}

	newClient := func() *client {
		clientID := make([]byte, 64)
		rand.Read(clientID)
		swarm, err := ppspp.NewSwarm(id, options)
		assert.NoError(t, err, "swarm constructor failed")
		return &client{
			id:    clientID,
			swarm: swarm,
		}
	}

	src, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: options,
		Key:          key,
	})
	assert.NoError(t, err, "writer constructor failed")

	clients := []*client{{swarm: src.Swarm()}}
	for i := 0; i < 5; i++ {
		clients = append(clients, newClient())
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	logger := ppspptest.Logger()

	for _, c := range clients {
		c.runner = ppspp.NewRunner(ctx, logger)
	}

	for i := 0; i < len(clients); i++ {
		for j := i + 1; j < len(clients); j++ {
			iConn, jConn := ppspptest.NewConnPair()

			iChannelReader, iPeer := clients[i].runner.RunPeer(clients[i].id, iConn)
			jCHannelReader, jPeer := clients[j].runner.RunPeer(clients[j].id, jConn)

			err := iPeer.RunSwarm(clients[i].swarm, 1, 1)
			assert.NoError(t, err, "channel open failed")
			err = jPeer.RunSwarm(clients[j].swarm, 1, 1)
			assert.NoError(t, err, "channel open failed")

			go ppspptest.ReadChannelConn(iConn, iChannelReader)
			go ppspptest.ReadChannelConn(jConn, jCHannelReader)
		}
	}

	go func() {
		ivl := 100 * time.Millisecond
		rate := 6000

		tc := time.NewTicker(ivl).C
		b := make([]byte, rate*1000/8/int(time.Second/ivl))
		for range tc {
			if _, err := src.Write(b); err != nil {
				log.Println(err)
			}
		}
	}()

	go func() {
		log.Println(http.ListenAndServe(":6061", nil))
	}()

	lastReads := make([]int64, len(clients))
	go func() {
		t := time.NewTicker(time.Second)
		for range t.C {
			times := make([]time.Duration, len(clients))
			for i := 1; i < len(clients); i++ {
				times[i] = time.Since(time.Unix(0, atomic.LoadInt64(&lastReads[i])))
			}
			log.Printf("time since last read: %s", times)
		}
	}()

	for i := 1; i < len(clients); i++ {
		go func(i int) {
			for {
				limit := int64(500000)
				n, err := io.CopyN(ioutil.Discard, clients[i].swarm.Reader(), limit)
				// assert.NoError(t, err, "read failed")
				// assert.Equal(t, limit, n, "short read")
				_, _ = n, err
				atomic.StoreInt64(&lastReads[i], time.Now().UnixNano())
			}
		}(i)
	}
	time.Sleep(30 * time.Second)

	// var wg sync.WaitGroup
	// for i := 1; i < len(clients); i++ {
	// 	wg.Add(1)
	// 	go func(i int) {
	// 		limit := int64(500000)
	// 		n, err := io.CopyN(ioutil.Discard, clients[i].swarm.Reader(), limit)
	// 		assert.NoError(t, err, "read failed")
	// 		assert.Equal(t, limit, n, "short read")
	// 		wg.Done()
	// 	}(i)
	// }
	// wg.Wait()
}
