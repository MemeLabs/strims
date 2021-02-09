package ppspp

import (
	"context"
	"fmt"
	"io"
	"log"
	"math/rand"
	"os"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/stretchr/testify/assert"
)

type testPeer struct {
	downloadRate int
	uploadRate   int
	city         ppspptest.City
}

func TestSwarmSim(t *testing.T) {
	byteRate := 8000 * ppspptest.Kbps
	writesPerSecond := 10

	bytesReadGoal := byteRate * 20

	peers := []testPeer{
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   10 * ppspptest.Mbps,
			city:         ppspptest.NewYork,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Boston,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Seattle,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.SanFrancisco,
		},
		// {
		// 	downloadRate: 150 * ppspptest.Mbps,
		// 	uploadRate:   15 * ppspptest.Mbps,
		// 	city:         ppspptest.LosAngeles,
		// },
		// {
		// 	downloadRate: 150 * ppspptest.Mbps,
		// 	uploadRate:   15 * ppspptest.Mbps,
		// 	city:         ppspptest.London,
		// },
		// {
		// 	downloadRate: 150 * ppspptest.Mbps,
		// 	uploadRate:   15 * ppspptest.Mbps,
		// 	city:         ppspptest.Berlin,
		// },
		// {
		// 	downloadRate: 150 * ppspptest.Mbps,
		// 	uploadRate:   15 * ppspptest.Mbps,
		// 	city:         ppspptest.Paris,
		// },
		// {
		// 	downloadRate: 150 * ppspptest.Mbps,
		// 	uploadRate:   15 * ppspptest.Mbps,
		// 	city:         ppspptest.Rome,
		// },
	}

	key := ppspptest.Key()
	id := NewSwarmID(key.Public)
	options := SwarmOptions{
		LiveWindow: 1 << 14,
	}

	type client struct {
		id        []byte
		city      ppspptest.City
		bandwidth *ppspptest.ConnThrottle
		swarm     *Swarm
		scheduler *Scheduler
		conns     []*ppspptest.MeterConn
	}

	newClient := func(p testPeer) *client {
		clientID := make([]byte, 64)
		rand.Read(clientID)
		swarm, err := NewSwarm(id, options)
		assert.NoError(t, err, "swarm constructor failed")
		return &client{
			id:        clientID,
			city:      p.city,
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
		city:      peers[0].city,
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
		c.scheduler.label = c.city.Name
	}

	f, _ := os.OpenFile(fmt.Sprintf("/tmp/capconn/log-%d.bin", time.Now().Unix()), os.O_CREATE|os.O_WRONLY, 0644)
	capLog := ppspptest.NewCapLogWriter(f)
	defer f.Close()

	for i := 0; i < len(clients); i++ {
		for j := i + 1; j < len(clients); j++ {
			iPeer := NewPeer(clients[i].id)
			jPeer := NewPeer(clients[j].id)

			clients[i].scheduler.AddPeer(ctx, iPeer)
			clients[j].scheduler.AddPeer(ctx, jPeer)

			iConn, jConn := ppspptest.NewConnPair()

			iConn = ppspptest.NewThrottleConn(iConn, clients[i].bandwidth)
			jConn = ppspptest.NewThrottleConn(jConn, clients[j].bandwidth)

			latency := ppspptest.ComputeLatency(clients[i].city.LatLng, clients[j].city.LatLng)
			iConn, jConn = ppspptest.NewLagConnPair(iConn, jConn, latency)

			iConn, err = ppspptest.NewCapConn(iConn, capLog.Writer(), fmt.Sprintf("%s : %s", clients[i].city.Name, clients[j].city.Name))
			assert.NoError(t, err, "cap conn open failed")
			jConn, err = ppspptest.NewCapConn(jConn, capLog.Writer(), fmt.Sprintf("%s : %s", clients[j].city.Name, clients[i].city.Name))
			assert.NoError(t, err, "cap conn open failed")

			imConn := ppspptest.NewMeterConn(iConn)
			jmConn := ppspptest.NewMeterConn(jConn)

			clients[i].conns[j] = imConn
			clients[j].conns[i] = jmConn

			iChan, err := OpenChannel(logger, iPeer, clients[i].swarm, imConn)
			assert.NoError(t, err, "channel open failed")
			jChan, err := OpenChannel(logger, jPeer, clients[j].swarm, jmConn)
			assert.NoError(t, err, "channel open failed")

			go ppspptest.ReadChannelConn(imConn, iChan)
			go ppspptest.ReadChannelConn(jmConn, jChan)
		}
	}

	go func() {
		b := make([]byte, byteRate/writesPerSecond)
		t := time.NewTicker(time.Second / time.Duration(writesPerSecond))
		var nn int
		for range t.C {
			n, _ := src.Write(b)
			if nn += n; nn >= bytesReadGoal*2 {
				break
			}
		}
	}()

	go func() {
		// f, err := os.OpenFile(fmt.Sprintf("./samples-%d.csv", time.Now().Unix()), os.O_CREATE|os.O_WRONLY, 0644)
		// assert.Nil(t, err, "log open failed")
		// defer f.Close()

		var prev []int64

		var labels strings.Builder
		labels.WriteString("tick")
		for i, c := range clients {
			// labels.WriteString(fmt.Sprintf(",%s", c.location))
			for j, conn := range c.conns {
				if conn != nil {
					// labels.WriteString(fmt.Sprintf(",%d:%d", i, j))
					labels.WriteString(fmt.Sprintf(",%s:%s", clients[i].city.Name, clients[j].city.Name))
					prev = append(prev, 0)
				}
			}
		}
		labels.WriteRune('\n')
		// _, err = f.WriteString(labels.String())
		assert.Nil(t, err, "writing string failed")

		// t := time.NewTicker(time.Second)
		ticker := time.NewTicker(time.Second)
		var tick int
		for range ticker.C {
			log.Println("=====================================================")

			var row strings.Builder
			row.WriteString(strconv.FormatInt(int64(tick), 10))
			var k int
			for i, c := range clients {
				c.swarm.bins.Lock()
				nextEmpty := c.swarm.bins.Requested.FindEmpty()
				lastFilled := c.swarm.bins.Requested.FindLastFilled()
				log.Printf("next empty bin: %s, lastFilled: %s", nextEmpty, lastFilled)
				c.swarm.bins.Unlock()

				log.Printf("%-16s%-20s %-12s %-7s %-12s %-12s %-7s %s", "from", "to", "bytes", "Bps", "%", "bytes", "Bps", "%")

				var rn, wn, rr, wr int64
				for j, conn := range c.conns {
					if conn != nil {
						rn += conn.ReadBytes()
						rr += conn.ReadByteRate()
						wn += conn.WrittenBytes()
						wr += conn.WriteByteRate()

						row.WriteString(fmt.Sprintf(",%d", conn.ReadBytes()-prev[k]))
						prev[k] = conn.ReadBytes()
						k++

						log.Printf(
							"%-16s%-16s in: %-12d %-7d %-7.2f out: %-12d %-7d %.2f",
							c.city.Name,
							clients[j].city.Name,
							conn.ReadBytes(),
							conn.ReadByteRate(),
							float64(conn.ReadByteRate())/float64(peers[i].downloadRate)*100,
							conn.WrittenBytes(),
							conn.WriteByteRate(),
							float64(conn.WriteByteRate())/float64(peers[i].uploadRate)*100,
						)
					}
				}
				log.Printf("%-32s in: %-12d %-15d out: %-12d %-7d", c.city.Name, rn, rr, wn, wr)
				log.Println("")

				// row.WriteString(fmt.Sprintf(",%d", wn-prev[i]))
				// prev[i] = wn
			}
			row.WriteRune('\n')
			// _, err = f.WriteString(row.String())
			assert.Nil(t, err, "writing string failed")

			// log.Println("---")
			tick++
		}
	}()

	var wg sync.WaitGroup
	for i := 1; i < len(clients); i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()

			var w testWriter
			t := time.NewTicker(time.Second)
			defer t.Stop()

			// go func() {
			// 	var prev uint64
			// 	for range t.C {
			// 		n := w.WrittenBytes()
			// 		log.Printf("%d read bytes: % -10d %t", i, n-prev, (n-prev) >= 437500)
			// 		prev = n
			// 	}
			// }()

			if _, err := io.CopyN(&w, clients[i].swarm.Reader(), int64(bytesReadGoal)); err != nil {
				log.Panicln(err)
			}
		}(i)
	}
	wg.Wait()
}

type testWriter struct {
	n uint64
}

func (w *testWriter) Write(p []byte) (int, error) {
	atomic.AddUint64(&w.n, uint64(len(p)))
	return len(p), nil
}

func (w *testWriter) WrittenBytes() uint64 {
	return atomic.LoadUint64(&w.n)
}
