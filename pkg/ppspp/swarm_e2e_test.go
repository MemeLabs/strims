package ppspp

import (
	"context"
	"io"
	"io/ioutil"
	"sync"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/tj/assert"
)

func TestSwarmE2E(t *testing.T) {
	key := ppspptest.Key()
	id := NewSwarmID(key.Public)
	options := SwarmOptions{
		LiveWindow: 1 << 12,
	}

	type client struct {
		swarm     *Swarm
		scheduler *Scheduler
	}

	newClient := func() *client {
		swarm, err := NewSwarm(id, options)
		assert.NoError(t, err, "swarm constructor failed")
		return &client{swarm: swarm}
	}

	src, err := NewWriter(WriterOptions{
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
		c.scheduler = NewScheduler(ctx, logger)
	}

	for i := 0; i < len(clients); i++ {
		for j := i + 1; j < len(clients); j++ {
			iPeer := NewPeer()
			jPeer := NewPeer()

			clients[i].scheduler.AddPeer(ctx, iPeer)
			clients[j].scheduler.AddPeer(ctx, jPeer)

			iConn, jConn := ppspptest.NewConnPair()

			iChan, err := OpenChannel(iPeer, clients[i].swarm, iConn)
			assert.NoError(t, err, "channel open failed")
			jChan, err := OpenChannel(jPeer, clients[j].swarm, jConn)
			assert.NoError(t, err, "channel open failed")

			go ppspptest.ReadChannelConn(iConn, iChan)
			go ppspptest.ReadChannelConn(jConn, jChan)
		}
	}

	go func() {
		tc := time.NewTicker(100 * time.Millisecond).C
		b := make([]byte, 75000)
		for range tc {
			src.Write(b)
		}
	}()

	var wg sync.WaitGroup
	for i := 1; i < len(clients); i++ {
		wg.Add(1)
		go func(i int) {
			limit := int64(500000)
			n, err := io.CopyN(ioutil.Discard, clients[i].swarm.Reader(), limit)
			assert.NoError(t, err, "read failed")
			assert.Equal(t, limit, n, "short read")
			wg.Done()
		}(i)
	}
	wg.Wait()
}
