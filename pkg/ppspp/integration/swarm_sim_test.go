package integration

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"log"
	"math/rand"
	"os"
	"path"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	pkgerrors "github.com/pkg/errors"

	"github.com/MemeLabs/go-ppspp/pkg/errutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/stretchr/testify/assert"
)

type testPeer struct {
	downloadRate int
	uploadRate   int
	city         ppspptest.City
	peers        testCityList
}

type testCityList []ppspptest.City

func (c testCityList) Contains(v ppspptest.City) bool {
	for i := range c {
		if c[i] == v {
			return true
		}
	}
	return false
}

func TestSwarmSim(t *testing.T) {
	byteRate := 6000 * ppspptest.Kbps
	writesPerSecond := 10

	bytesReadGoal := byteRate * 20

	peers := []testPeer{
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
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
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.LosAngeles,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.London,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Berlin,
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Paris,
			peers:        testCityList{ppspptest.LosAngeles, ppspptest.London, ppspptest.Berlin, ppspptest.Rome, ppspptest.HongKong, ppspptest.Moscow, ppspptest.Tokyo, ppspptest.Singapore},
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Rome,
			peers:        testCityList{ppspptest.LosAngeles, ppspptest.London, ppspptest.Berlin, ppspptest.Paris, ppspptest.HongKong, ppspptest.Moscow, ppspptest.Tokyo, ppspptest.Singapore},
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.HongKong,
			peers:        testCityList{ppspptest.LosAngeles, ppspptest.London, ppspptest.Berlin, ppspptest.Paris, ppspptest.Rome, ppspptest.Moscow, ppspptest.Tokyo, ppspptest.Singapore},
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Moscow,
			peers:        testCityList{ppspptest.Seattle, ppspptest.Berlin, ppspptest.Paris, ppspptest.Rome, ppspptest.HongKong, ppspptest.Tokyo, ppspptest.Singapore},
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Tokyo,
			peers:        testCityList{ppspptest.SanFrancisco, ppspptest.Paris, ppspptest.Rome, ppspptest.HongKong, ppspptest.Moscow, ppspptest.Singapore},
		},
		{
			downloadRate: 150 * ppspptest.Mbps,
			uploadRate:   15 * ppspptest.Mbps,
			city:         ppspptest.Singapore,
			peers:        testCityList{ppspptest.Paris, ppspptest.Rome, ppspptest.HongKong, ppspptest.Moscow, ppspptest.Tokyo},
		},
	}

	key := ppspptest.Key()
	id := ppspp.NewSwarmID(key.Public)
	options := ppspp.SwarmOptions{
		LiveWindow:  1 << 14,
		StreamCount: 16,
	}

	type client struct {
		id        []byte
		city      ppspptest.City
		bandwidth *ppspptest.ConnThrottle
		swarm     *ppspp.Swarm
		runner    *ppspp.Runner
		conns     []*ppspptest.MeterConn
		qos       *qos.Control
		writer    testWriter
	}

	var nextClientID uint16
	newClientID := func() []byte {
		nextClientID++
		id := make([]byte, 2)
		binary.BigEndian.PutUint16(id, nextClientID)
		return id
	}

	pairID := func(a, b []byte) []byte {
		c := make([]byte, len(a)+len(b))
		n := copy(c, a)
		copy(c[n:], b)
		return c
	}

	newClient := func(p testPeer) *client {
		swarm, err := ppspp.NewSwarm(id, options)
		assert.NoError(t, err, "swarm constructor failed")
		return &client{
			id:        newClientID(),
			city:      p.city,
			bandwidth: ppspptest.NewConnThrottle(p.downloadRate, p.uploadRate),
			swarm:     swarm,
			conns:     make([]*ppspptest.MeterConn, len(peers)),
			qos:       qos.NewWithLimit(uint64(p.uploadRate)),
		}
	}

	src, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: options,
		Key:          key,
	})
	assert.NoError(t, err, "writer constructor failed")

	clients := []*client{{
		id:        newClientID(),
		city:      peers[0].city,
		bandwidth: ppspptest.NewConnThrottle(peers[0].downloadRate, peers[0].uploadRate),
		swarm:     src.Swarm(),
		conns:     make([]*ppspptest.MeterConn, len(peers)),
		qos:       qos.NewWithLimit(uint64(peers[0].uploadRate)),
	}}
	for i := 1; i < len(peers); i++ {
		clients = append(clients, newClient(peers[i]))
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	logger := ppspptest.Logger()

	for _, c := range clients {
		c.runner = ppspp.NewRunner(ctx, logger)
		// c.runner.label = c.city.Name
	}

	if _, err := os.Stat(ppspptest.CapConnLogDir()); os.IsNotExist(err) {
		assert.NoError(t, os.MkdirAll(ppspptest.CapConnLogDir(), 0755))
	}
	capLogPath := path.Join(ppspptest.CapConnLogDir(), time.Now().Format(time.RFC3339)+ppspptest.CapLogExt)
	log.Println("writing to cap log:", capLogPath)
	f, err := os.OpenFile(fmt.Sprintf("%s.tmp", capLogPath), os.O_CREATE|os.O_WRONLY, 0644)
	assert.NoError(t, err, "capconn log open failed")
	capLog := ppspptest.NewCapLogWriter(f)
	defer func() {
		assert.NoError(t, capLog.Close())
		assert.NoError(t, f.Close())
		assert.NoError(t, os.Rename(f.Name(), capLogPath))
	}()

	done := make(chan struct{})
	var doneOnce sync.Once

	runSafely := func(fn func()) {
		defer func() {
			if err := errutil.RecoverError(recover()); err != nil {
				fmt.Println(pkgerrors.WithStack(err))
			}
			doneOnce.Do(func() { close(done) })
		}()
		fn()
	}

	for i := 0; i < len(clients); i++ {
		for j := i + 1; j < len(clients); j++ {
			iConn, jConn := ppspptest.NewConnPair()

			iConn = ppspptest.NewThrottleConn(iConn, clients[i].bandwidth)
			jConn = ppspptest.NewThrottleConn(jConn, clients[j].bandwidth)

			latency := ppspptest.ComputeLatency(clients[i].city.LatLng, clients[j].city.LatLng)
			iConn, jConn = ppspptest.NewLagConnPair(iConn, jConn, latency)

			iConn = ppspptest.NewQOSConn(iConn, clients[i].qos.AddSession(qos.MaxWeight))
			jConn = ppspptest.NewQOSConn(jConn, clients[j].qos.AddSession(qos.MaxWeight))

			// iConn, err = ppspptest.NewCapConn(iConn, capLog.Writer(), fmt.Sprintf("%s : %s", clients[i].city.Name, clients[j].city.Name))
			// assert.NoError(t, err, "cap conn open failed")
			// jConn, err = ppspptest.NewCapConn(jConn, capLog.Writer(), fmt.Sprintf("%s : %s", clients[j].city.Name, clients[i].city.Name))
			// assert.NoError(t, err, "cap conn open failed")

			imConn := ppspptest.NewMeterConn(iConn)
			jmConn := ppspptest.NewMeterConn(jConn)

			clients[i].conns[j] = imConn
			clients[j].conns[i] = jmConn

			if (peers[i].peers == nil || peers[i].peers.Contains(peers[j].city)) && (peers[j].peers == nil || peers[j].peers.Contains(peers[i].city)) {
				iChanReader, iPeer := clients[i].runner.RunPeer(pairID(clients[i].id, clients[j].id), imConn)
				jChanReader, jPeer := clients[j].runner.RunPeer(pairID(clients[j].id, clients[i].id), jmConn)

				err = clients[i].runner.RunChannel(clients[i].swarm, iPeer, codec.Channel(i), codec.Channel(j))
				assert.NoError(t, err, "channel open failed")
				err = clients[j].runner.RunChannel(clients[j].swarm, jPeer, codec.Channel(j), codec.Channel(i))
				assert.NoError(t, err, "channel open failed")

				go ppspptest.ReadChannelConn(imConn, iChanReader)
				go ppspptest.ReadChannelConn(jmConn, jChanReader)
			}
		}
	}

	go runSafely(func() {
		b := make([]byte, byteRate/writesPerSecond)
		t := time.NewTicker(time.Second / time.Duration(writesPerSecond))
		var nn int
		for range t.C {
			rand.Read(b)
			n, _ := src.Write(b)
			if nn += n; nn >= bytesReadGoal*2 {
				break
			}
		}
	})

	start := time.Now()
	go runSafely(func() {
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
		ticker := time.NewTicker(1 * time.Second)
		var tick int
		for range ticker.C {
			log.Printf("=== %-14s =====================================================", time.Since(start))

			var row strings.Builder
			row.WriteString(strconv.FormatInt(int64(tick), 10))
			var k int
			for i, c := range clients {
				// c.swarm.bins.Lock()
				// nextEmpty := c.swarm.bins.Requested.FindEmpty()
				// lastFilled := c.swarm.bins.Requested.FindLastFilled()
				// log.Printf("next empty bin: %s, lastFilled: %s", nextEmpty, lastFilled)
				// c.swarm.bins.Unlock()

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
				log.Printf("%-32s in: %-12d %-7d %-7.2f out: %-12d %-7d %.2f",
					c.city.Name,
					rn,
					rr,
					float64(rr)/float64(peers[i].downloadRate)*100,
					wn,
					wr,
					float64(wr)/float64(peers[i].uploadRate)*100,
				)
				log.Printf("%-26s readable: %-12d", c.city.Name, c.writer.WrittenBytes())
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
	})

	var wg sync.WaitGroup
	wg.Add(len(clients))
	for _, c := range clients {
		c := c
		go runSafely(func() {
			defer wg.Done()

			n := int64(bytesReadGoal)
			for n > 0 {
				nn, err := io.CopyN(&c.writer, c.swarm.Reader(), int64(n))
				n -= nn
				if errors.Is(err, store.ErrBufferUnderrun) {
					skipped, err := c.swarm.Reader().Recover()
					if err != nil {
						log.Panic(err)
					}
					log.Printf("recoverred from buffer underrun. skipped %d bytes", skipped)
				} else if err != nil {
					log.Panicln(err)
				}
			}
		})
	}

	go func() {
		wg.Wait()
		doneOnce.Do(func() { close(done) })
	}()

	<-done
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
