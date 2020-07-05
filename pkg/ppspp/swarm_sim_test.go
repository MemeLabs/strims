package ppspp

import (
	"context"
	"log"
	"sync"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/golang/geo/s2"
	"github.com/tj/assert"
)

const (
	earthRadius = 6378.1370
	c           = 300000
	linkSpeed   = c / 10
)

type peer struct {
	downloadRate int
	uploadRate   int
	location     s2.LatLng
}

func TestSwarmSim(t *testing.T) {
	peers := []peer{
		{
			downloadRate: 300 * ppspptest.Mbps,
			uploadRate:   30 * ppspptest.Mbps,
			// new york
			location: s2.LatLngFromDegrees(40.7128, -74.0060),
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   150 * ppspptest.Mbps,
			// boston
			location: s2.LatLngFromDegrees(42.3601, -71.0589),
		},
		{
			downloadRate: 60 * ppspptest.Mbps,
			uploadRate:   3 * ppspptest.Mbps,
			// seattle
			location: s2.LatLngFromDegrees(47.6062, -122.3321),
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   6 * ppspptest.Mbps,
			// san francisco
			location: s2.LatLngFromDegrees(37.7749, -122.4194),
		},
		{
			downloadRate: 250 * ppspptest.Mbps,
			uploadRate:   25 * ppspptest.Mbps,
			// los angeles
			location: s2.LatLngFromDegrees(34.0522, -118.2437),
		},
	}

	key := ppspptest.Key()
	id := NewSwarmID(key.Public)
	options := SwarmOptions{
		LiveWindow: 1 << 22,
	}

	type client struct {
		location  s2.LatLng
		bandwidth *ppspptest.ConnThrottle
		swarm     *Swarm
		scheduler *Scheduler
		conns     []*ppspptest.MeterConn
	}

	newClient := func(p peer) *client {
		swarm, err := NewSwarm(id, options)
		assert.NoError(t, err, "swarm constructor failed")
		return &client{
			location:  p.location,
			bandwidth: ppspptest.NewConnThrottle(p.downloadRate, p.uploadRate),
			swarm:     swarm,
			conns:     make([]*ppspptest.MeterConn, len(peers)),
		}
	}

	src, err := NewWriter(WriterOptions{
		SwarmOptions: options,
		Key:          key,
	})
	assert.NoError(t, err, "writer constructor failed")

	clients := []*client{{
		location:  peers[0].location,
		bandwidth: ppspptest.NewConnThrottle(peers[0].downloadRate, peers[0].uploadRate),
		swarm:     src.Swarm(),
		conns:     make([]*ppspptest.MeterConn, len(peers)),
	}}
	for i := 1; i < len(peers); i++ {
		clients = append(clients, newClient(peers[i]))
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

			iConn = ppspptest.NewThrottleConn(iConn, clients[i].bandwidth)
			jConn = ppspptest.NewThrottleConn(jConn, clients[j].bandwidth)

			d := clients[i].location.Distance(clients[j].location).Radians() * earthRadius
			latency := time.Duration(float64(time.Second) * d / linkSpeed)
			iConn, jConn = ppspptest.NewLagConnPair(iConn, jConn, latency)

			imConn := ppspptest.NewMeterConn(iConn)
			jmConn := ppspptest.NewMeterConn(jConn)

			clients[i].conns[j] = imConn
			clients[j].conns[i] = jmConn

			iChan, err := OpenChannel(iPeer, clients[i].swarm, imConn)
			assert.NoError(t, err, "channel open failed")
			jChan, err := OpenChannel(jPeer, clients[j].swarm, jmConn)
			assert.NoError(t, err, "channel open failed")

			go ppspptest.ReadChannelConn(imConn, iChan)
			go ppspptest.ReadChannelConn(jmConn, jChan)
		}
	}

	go func() {
		t := time.NewTicker(100 * time.Millisecond)
		b := make([]byte, 75000)
		var nn int
		for range t.C {
			n, _ := src.Write(b)
			if nn += n; nn >= 10000000 {
				break
			}
		}
	}()

	go func() {
		t := time.NewTicker(time.Second)
		for range t.C {
			log.Println("---")
			for i, c := range clients {
				for j, conn := range c.conns {
					if i != j {
						log.Printf("conn: %d:%d, in: %d, out: %d", i, j, conn.ReadBytes(), conn.WrittenBytes())
					}
				}
			}
			log.Println("---")
		}
	}()

	var wg sync.WaitGroup
	for i := 1; i < len(clients); i++ {
		wg.Add(1)
		go func(i int) {
			var nn int
			r := clients[i].swarm.Reader()
			b := make([]byte, 1024)
			for nn < 5000000 {
				n, err := r.Read(b)
				if err != nil {
					panic(err)
				}
				// log.Println("read from", i, n, nn)
				nn += n
			}
			// log.Println("done --------------", i)

			// limit := int64(500000)
			// n, err := io.CopyN(ioutil.Discard, clients[i].swarm.Reader(), limit)
			// assert.NoError(t, err, "read failed")
			// assert.Equal(t, limit, n, "short read")
			wg.Done()
		}(i)
	}
	wg.Wait()
}
