package ppspp

import (
	"context"
	"fmt"
	"log"
	"math/rand"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"go.uber.org/zap"
)

const planForIntervals = 2

// NewScheduler ...
func NewScheduler(ctx context.Context, logger *zap.Logger) (s *Scheduler) {
	s = &Scheduler{
		logger: logger,

		selector: &Test3ChunkSelector{
			seed: rand.Uint64(),
		},
	}

	return
}

// Scheduler ...
type Scheduler struct {
	logger *zap.Logger
	close  sync.Map
	sent   int64

	label string

	selector *Test3ChunkSelector
}

const (
	defaultWriteInterval = 200 * time.Millisecond
	maxWriteInterval     = time.Second
	minWriteInterval     = 50 * time.Millisecond
	rescheduleInterval   = 250 * time.Millisecond
)

// AddPeer ...
func (r *Scheduler) AddPeer(ctx context.Context, peer *Peer) {
	ctx, close := context.WithCancel(ctx)
	r.close.Store(peer, close)

	go func() {
		writeTimer := time.NewTimer(defaultWriteInterval)
		defer writeTimer.Stop()

		for {
			select {
			case t := <-writeTimer.C:
				r.runPeer(peer, iotime.FromTime(t))
				writeTimer.Reset(r.peerNextTickDuration(peer))
				// writeTimer.Reset(minWriteInterval)
			case <-ctx.Done():
				return
			}
		}
	}()
}

func (r *Scheduler) logf(format string, v ...interface{}) {
	if r.label == "New York" || r.label == "Boston" {
		return
	}
	fmt.Printf("%s ", r.label)
	log.Printf(format, v...)
}

func (r *Scheduler) logln(v ...interface{}) {
	if r.label == "New York" || r.label == "Boston" {
		return
	}
	fmt.Printf("%s ", r.label)
	log.Println(v...)
}

func (r *Scheduler) peerNextTickDuration(peer *Peer) time.Duration {
	peer.Lock()
	rttMean, _, _ := peer.rtt.Value()
	peer.Unlock()

	rtt := time.Duration(rttMean)
	if rtt < minWriteInterval {
		return minWriteInterval
	} else if rtt > maxWriteInterval {
		return maxWriteInterval
	}
	return rtt
}

// RemovePeer ...
func (r *Scheduler) RemovePeer(peer *Peer) {
	close, ok := r.close.Load(peer)
	if ok {
		close.(context.CancelFunc)()
		r.close.Delete(peer)
	}
}

var glock sync.Mutex

func (r *Scheduler) runPeer(p *Peer, t time.Time) {
	p.Lock()
	defer p.Unlock()

	// for s := range p.channels {
	// 	s.bins.Lock()
	// 	rttMean, rttVar, _ := s.bins.rtt.Value()
	// 	if rttMean > 0 {
	// 		s.bins.selector.Label = r.label
	// 		ok := s.bins.selector.SetVPH(time.Duration(rttMean), time.Duration(math.Sqrt(rttVar)), s.bins.Requested)
	// 		if ok {
	// 			first := s.bins.Requested.FindEmpty()
	// 			if s.bins.Requested.Filled() {
	// 				first = s.bins.Requested.RootBin().BaseRight() + 2
	// 			}
	// 			last := s.bins.Available.FindLastFilled()

	// 			s.bins.selector.SetWanted(time.Duration(rttMean), time.Duration(math.Sqrt(rttVar)), first, last)
	// 			// glock.Lock()
	// 			// fmt.Printf("%s %d %d ", r.label, first, last)
	// 			// s.bins.selector.DebugPrint(first, last)
	// 			// glock.Unlock()
	// 		}
	// 	}
	// 	s.bins.Unlock()
	// }

	r.sendPongs(p, t)
	r.sendPeerTimeouts(p, t)
	// r.sendPeerExchange(p, w)
	r.sendPeerData(p, t)
	r.sendPeerPing(p, t)

	// if err := w.Flush(); err != nil {
	// 	r.logger.Debug("flush failed", zap.Error(err))
	// }

	// TODO: only flush/push the last
	for _, c := range p.channels {
		if err := c.w.Flush(); err != nil {
			c.Close()
		}
	}
}

func (r *Scheduler) sendPeerTimeouts(p *Peer, t time.Time) {
	// TODO: separate read and write timeouts
	// TODO: mediate cancel/retry floods from increased latency
	// deadline := t.Add(-p.ledbat.CTO() * planForIntervals)

	// add rtt...
	deadline := t.Add(-time.Duration(p.rtt.Mean()+p.rtt.StdDev()*2) * 2)

	var nn int
	for s, c := range p.channels {
		c.Lock()

		// for i := c.sentBinHistory.IterateUntil(deadline); i.Next(); {
		// 	if c.unackedBins.FilledAt(i.Bin()) {
		// 		// p.ledbat.AddDataLoss(int(i.Bin().BaseLength())*s.chunkSize(), false)
		// 		c.unackedBins.Reset(i.Bin())
		// 	}
		// }

		for i := c.requestedBinHistory.IterateUntil(deadline); i.Next(); {
			if !s.store.FilledAt(i.Bin()) {
				for b := i.Bin().BaseLeft(); b <= i.Bin().BaseRight(); b += 2 {
					if s.store.EmptyAt(b) {
						s.bins.Lock()
						s.bins.Requested.Reset(b)
						s.bins.Unlock()
						p.addCancelledChunk()
					}
					nn++
				}

				if _, err := c.WriteCancel(codec.Cancel{Address: codec.Address(i.Bin())}); err != nil {
					r.logger.Debug("write failed", zap.Error(err))
				}
			}
		}

		c.Unlock()
	}
	if nn > 0 {
		r.logger.Debug(
			"cancel",
			zap.Int("bins", nn),
		)
	}
}

func (r *Scheduler) sendPeerData(p *Peer, t time.Time) {
	requesteCapacity := r.peerRequestCapacity(p)
	for s, c := range p.channels {
		c.Lock()

		if c.choked {
			c.Unlock()
			continue
		}

		// TODO: compress ACKs like HAVEs... min(delay sample)
		// for _, a := range c.acks {
		// 	c.WriteAck(a)
		// }
		c.acks = c.acks[:0]

		// TODO: avoid holding swarm lock during io...

		// TODO: move to channel and execute under addedBinsLock
		// TODO: move insert added bins to channel to resolve Swarm WriteChunk lock inconsistency
		for {
			b := c.addedBins.FindFilled()
			if b.IsNone() {
				break
			}
			b = s.store.Cover(b)
			if b.IsAll() {
				break
			}
			c.addedBins.Reset(b)

			if _, err := c.WriteHave(codec.Have{Address: codec.Address(b)}); err != nil {
				r.logger.Debug("write failed", zap.Error(err))
			}
		}

		requestBins, n := r.requestBins(requesteCapacity, s, c.channel)
		requesteCapacity -= n

		maxOverhead := 384

		for _, b := range requestBins {
			if _, err := c.WriteRequest(codec.Request{Address: codec.Address(b)}); err != nil {
				r.logger.Debug("write failed", zap.Error(err))
			}
			p.addRequestedChunks(b.BaseLength())
			c.requestedBinHistory.Push(b, t)
			p.trackBinRTT(c.id, b, t)
		}

		maxChunksPerData := 8
		b := pool.Get(uint16(maxChunksPerData * s.chunkSize()))
		// b := pool.Get(uint16(c.Cap()))

		var nw, no int
		// TODO: rlock s.chunks here
		// for p.ledbat.FlightSize() < p.ledbat.CWND() {
		for {
			rb := c.requestedBins.FindFilled()
			if rb.IsNone() {
				break
			}
			// TODO: limit with CWND/MTU/free bytes in frame
			if s.chunkSize() > c.Cap()-c.Len()-maxOverhead {
				c.Flush()
			}
			for int(rb.BaseLength()) > maxChunksPerData || int(rb.BaseLength())*s.chunkSize() > c.Cap()-c.Len()-maxOverhead {
				rb = rb.Left()
			}
			// rb = rb.BaseLeft()
			c.requestedBins.Reset(rb)

			if ok := s.store.ReadBin(rb, *b); ok {
				if _, err := s.verifier.WriteIntegrity(rb, c.availableBins, c); err != nil {
					r.logger.Debug("write failed", zap.Error(err))
				}

				// TODO: avoid writing data until after this?
				if _, err := c.WriteData(codec.Data{
					Address:   codec.Address(rb),
					Timestamp: codec.Timestamp{Time: t},
					Data:      codec.Buffer((*b)[:int(rb.BaseLength())*s.chunkSize()]),
				}); err != nil {
					r.logger.Debug("write failed", zap.Error(err))
				}

				// TODO: re-add with merged acks
				// p.trackBinRTT(c.id, rb, t)
				// p.ledbat.AddSent(s.chunkSize())
				// c.unackedBins.Set(rb)
				// c.sentBinHistory.Push(rb, t)
				nw++
				// atomic.AddInt64(&r.sent, 1)
			} else {
				no++
			}
		}

		//if atomic.LoadInt64(&r.sent) > 100 {
		// r.logger.Debug(
		// 	"data",
		// 	zap.Int("sent", nw),
		// 	zap.Int("missing", no),
		// 	zap.Int("flightSize", p.ledbat.FlightSize()),
		// 	zap.Int("cwnd", p.ledbat.CWND()),
		// )
		//}

		pool.Put(b)

		c.Unlock()
	}
}

func (r *Scheduler) sendPongs(p *Peer, t time.Time) {
	for _, c := range p.channels {
		if p := c.dequeuePong(); p != nil {
			if _, err := c.WritePong(*p); err != nil {
				r.logger.Debug("write failed", zap.Error(err))
			}
		}
	}
}

func (r *Scheduler) sendPeerPing(p *Peer, t time.Time) {
	for _, c := range p.channels {
		if c.Dirty() {
			if nonce, ok := p.trackPingRTT(c.id, t); ok {
				if _, err := c.WritePing(codec.Ping{Nonce: codec.Nonce{Value: nonce}}); err != nil {
					r.logger.Debug("write failed", zap.Error(err))
				}
			}
		}
	}
}

// func (r *Scheduler) peerRequestCapacity(p *Peer) int {
// 	p.ledbat.DigestDelaySamples()

// 	planForDuration := p.ledbat.RTTMean()
// 	// regardless of how fast our peer is sending us data we're not going to
// 	// send another request for at least minWriteInterval...
// 	// TODO: this is repeated below in scheduler, maybe fix that...
// 	if planForDuration < minWriteInterval {
// 		planForDuration = minWriteInterval
// 	}
// 	planForDuration *= planForIntervals
// 	// if planForDuration > time.Second {
// 	// 	planForDuration = time.Second
// 	// }

// 	var capacity int
// 	chunkInterval := p.chunkInterval()
// 	if chunkInterval != 0 {
// 		capacity = int(planForDuration / chunkInterval)
// 	}
// 	capacity -= p.outstandingChunks()
// 	if capacity < planForIntervals {
// 		capacity = planForIntervals * 120
// 	}

// 	//if chunkInterval != 0 {
// 	// r.logger.Debug(
// 	// 	"capacity",
// 	// 	zap.Int("capacity", capacity),
// 	// 	zap.Duration("p.ledbat.RTTMean()", p.ledbat.RTTMean()),
// 	// 	zap.Duration("planforDuration", planForDuration),
// 	// 	zap.Duration("chunkInterval", chunkInterval),
// 	// )
// 	//}

// 	return capacity
// }

// func (r *Scheduler) peerRequestCapacity(p *Peer) int {
// 	// p.ledbat.DigestDelaySamples()

// 	// this is the part where we use etcp

// 	planForDuration := time.Duration(p.rtt.Mean() * 2)
// 	// regardless of how fast our peer is sending us data we're not going to
// 	// send another request for at least minWriteInterval...
// 	// TODO: this is repeated below in scheduler, maybe fix that...
// 	if planForDuration < minWriteInterval {
// 		planForDuration = minWriteInterval * 2
// 	}
// 	// planForDuration *= planForIntervals
// 	// if planForDuration > time.Second {
// 	// 	planForDuration = time.Second
// 	// }

// 	var capacity int
// 	chunkInterval := p.chunkInterval()
// 	if chunkInterval != 0 {
// 		capacity = int(planForDuration/chunkInterval) + 1
// 		// r.logf("cap: %d, planForDuration: %s, chunkInterval: %s", capacity, planForDuration, chunkInterval)
// 	}

// 	if capacity < 8 {
// 		capacity = 8
// 	}

// 	capacity -= p.outstandingChunks()

// 	// capacity -= p.outstandingChunks()
// 	// if capacity < planForIntervals {
// 	// 	capacity = planForIntervals * 120
// 	// }

// 	//if chunkInterval != 0 {
// 	// r.logger.Debug(
// 	// 	"capacity",
// 	// 	zap.Int("capacity", capacity),
// 	// 	zap.Duration("p.ledbat.RTTMean()", p.ledbat.RTTMean()),
// 	// 	zap.Duration("planforDuration", planForDuration),
// 	// 	zap.Duration("chunkInterval", chunkInterval),
// 	// )
// 	//}

// 	return capacity
// }

func (r *Scheduler) peerRequestCapacity(p *Peer) int {
	return p.cwnd.CWND() - p.outstandingChunks()
}

func (r *Scheduler) requestBins(count int, s *Swarm, c *channel) ([]binmap.Bin, int) {
	s.bins.Lock()
	defer s.bins.Unlock()

	var bins []binmap.Bin
	var n int

	if !s.bins.Requested.Empty() {
		bins, n = r.selector.SelectBins(count, s.store.Bins(), s.bins.Available, s.bins.Requested, c.availableBins)
	} else {
		bins, n = (&FirstChunkSelector{}).SelectBins(count, s.bins.Available, s.bins.Requested, c.availableBins)
		if len(bins) != 0 {
			s.store.SetOffset(bins[0].BaseLeft() + 2)
		}
	}

	return bins, n
}

// ChunkSelector ...
type ChunkSelector interface {
	SelectBins(count int, seen, requested, available *binmap.Map) ([]binmap.Bin, int)
}
