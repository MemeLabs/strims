package encoding

import (
	"context"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
)

// Priority ...
type Priority int

// priorities
const (
	High Priority = iota
	Medium
	Low
	Skip
)

// Prioritizer ...
type Prioritizer interface {
	Prioritize(bin uint32) Priority
}

// MemePrioritizer ...
type MemePrioritizer struct {
}

// Prioritize ...
func (p *MemePrioritizer) Prioritize(bin uint32) Priority {
	return High
}

// NewScheduler ...
func NewScheduler(ctx context.Context) (s *Scheduler) {
	s = &Scheduler{}

	return
}

// Scheduler ...
type Scheduler struct {
	host  *Host
	close sync.Map
}

const (
	defaultWriteInterval = 200 * time.Millisecond
	maxWriteInterval     = time.Second
	minWriteInterval     = 10 * time.Millisecond
	rescheduleInterval   = 200 * time.Millisecond
)

// AddPeer ...
func (s *Scheduler) AddPeer(ctx context.Context, peer *Peer) {
	ctx, close := context.WithCancel(ctx)
	s.close.Store(peer, close)

	go func() {
		rescheduleTicker := time.NewTicker(rescheduleInterval)
		debugTicker := time.NewTicker(2 * time.Second)

		// TODO: cgo hack until tickers are fixed? https://github.com/golang/go/issues/27707
		writeInterval := defaultWriteInterval
		writeTicker := time.NewTicker(writeInterval)

		w := NewMemeWriter(peer.conn)

		for {
			select {
			case <-writeTicker.C:
				s.runPeer(peer, w)
			case <-ctx.Done():
				return
			case <-rescheduleTicker.C:
				newWriteInterval := s.peerWriteInterval(peer)
				// log.Println("newWriteInterval", unsafe.Pointer(peer), newWriteInterval)
				if writeInterval != newWriteInterval {
					debug.Yellow("new write interval", newWriteInterval)
					writeTicker.Stop()
					writeTicker = time.NewTicker(newWriteInterval)
					writeInterval = newWriteInterval
				}
			case <-debugTicker.C:
				s.printPeerDebugLog(peer)
			}
		}
	}()
}

// RemovePeer ...
func (s *Scheduler) RemovePeer(peer *Peer) {
	close, ok := s.close.Load(peer)
	if ok {
		close.(context.CancelFunc)()
		s.close.Delete(peer)
	}
}

func (s *Scheduler) peerWriteInterval(peer *Peer) (i time.Duration) {
	peer.Lock()
	// writeInterval := peer.ledbat.RTTMean() / time.DuRration(peer.ledbat.CWND() )
	i = peer.ledbat.RTTMean()
	peer.Unlock()

	if i < minWriteInterval {
		return minWriteInterval
	} else if i > maxWriteInterval {
		return maxWriteInterval
	}
	return
}

func (s *Scheduler) printPeerDebugLog(peer *Peer) {
	return

	peer.Lock()
	// spew.Dump(peer)
	if peer.ledbat.Debug() {
		// if peer.requestedChunkCount > 0 {
		channels := []string{}
		peer.channels.Range(func(key interface{}, value interface{}) bool {
			c := value.(*channel)
			c.swarm.Lock()
			c.Lock()

			// channels = append(channels, spew.Sdump([]interface{}{
			// 	"id", c.id,
			// 	// "swarm", c.swarm,
			// 	// "peer",  c.peer,
			// 	// "conn",      c.conn,
			// 	// "addedBins", c.addedBins,
			// 	// "requestedBins", c.requestedBins,
			// 	// "availableBins",       c.availableBins,
			// 	// "unackedBins",         c.unackedBins,
			// 	// "rttSampleBin",        c.rttSampleBin,
			// 	"sentBinHistory.ring", c.sentBinHistory.ring,
			// 	"requestedBinHistory.ring", c.requestedBinHistory.ring,
			// }))

			c.Unlock()
			c.swarm.Unlock()

			return true
		})

		log.Println(
			// "peer", unsafe.Pointer(peer),
			"\nrequested", peer.requestedChunkCount,
			"\nreceived", peer.receivedChunkCount,
			"\ncancelled", peer.cancelledChunkCount,
			"\nsent", peer.sentChunkCount,
			"\nacked", peer.ackedChunkCount,
			"\nnewly acked", peer.NewlyAckedCount(),
			"\nlost", peer.sentChunkCount-peer.ackedChunkCount,
			"\nOutstandingChunks", peer.OutstandingChunks(),
			"\nChunkIntervalMean", peer.ChunkIntervalMean(),
			"\nledbat.CWND()", peer.ledbat.CWND(),
			"\nledbat.CTO()", peer.ledbat.CTO(),
			"\nledbat.RTTMean()", peer.ledbat.RTTMean(),
			"\nledbat.FlightSize()", peer.ledbat.FlightSize(),
			"\ncwnd count", peer.ledbat.CWND()/ChunkSize,
			"\nflight count", peer.ledbat.FlightSize()/ChunkSize,
			// "loss", 1-float64(peer.receivedChunkCount)/float64(peer.requestedChunkCount),
			// "ledbat", spew.Sdump(peer.ledbat),
			// "peer", spew.Sdump(peer),
			"\nchannels", channels,
			"\n---",
		)
	}
	peer.Unlock()
}

func (s *Scheduler) runPeer(p *Peer, w *MemeWriter) {
	p.Lock()
	defer p.Unlock()

	// TODO: separate read and write timeouts
	// timeout := time.Now().Add(-p.ledbat.CTO() * 2)
	timeout := time.Now().Add(-2 * time.Second)
	_ = timeout

	p.channels.Range(func(_ interface{}, ci interface{}) bool {
		c := ci.(*channel)
		c.Lock()
		c.swarm.Lock()

		w.BeginFrame(c.remoteID)

		for i := c.sentBinHistory.IterateUntil(timeout); i.Next(); {
			if c.unackedBins.FilledAt(i.Bin()) {
				c.peer.ledbat.AddDataLoss(ChunkSize, false)
				c.unackedBins.Reset(i.Bin())
			}
		}

		for i := c.requestedBinHistory.IterateUntil(timeout); i.Next(); {
			if !c.swarm.loadedBins.FilledAt(i.Bin()) {
				for b := i.Bin().BaseLeft(); b <= i.Bin().BaseRight(); b += 2 {
					if c.swarm.loadedBins.EmptyAt(b) {
						c.swarm.requestedBins.Reset(b)
						c.peer.AddCancelledChunk()
					}
				}

				w.Write(&Cancel{Address(i.Bin())})
			}
		}

		c.swarm.Unlock()
		c.Unlock()

		return true
	})

	requesteCapacity := s.peerRequestCapacity(p)
	p.channels.Range(func(_ interface{}, ci interface{}) bool {
		c := ci.(*channel)
		c.Lock()

		if c.choked {
			return true
		}

		c.swarm.Lock()
		// TODO: avoid holding swarm lock during io...

		w.BeginFrame(c.remoteID)

		// TODO: compress ACKs like HAVEs... min(delay sample)
		for _, a := range c.acks {
			w.Write(&a)
		}
		c.acks = c.acks[:0]

		for _, b := range s.channelAddedBins(c) {
			w.Write(&Have{Address(b)})
		}

		requestBins, n := s.requestBins(requesteCapacity, c)
		requesteCapacity -= n

		for _, b := range requestBins {
			w.Write(&Request{Address(b)})
			p.AddRequestedChunks(b.BaseLength())
			c.requestedBinHistory.Push(b)
		}

		// TODO: rlock c.swarm.chunks here
		for p.ledbat.FlightSize() < p.ledbat.CWND() {
			rb := c.requestedBins.FindFilled()
			if rb.IsNone() {
				break
			}
			rb = rb.BaseLeft()
			c.requestedBins.Reset(rb)

			b, ok := c.swarm.chunks.Find(rb)
			if ok {
				// TODO: probably remove? with delayed acks these rtts aren't accurate
				// p.TrackBinRTT(c.id, rb)

				// TODO: avoid writing data until after this?
				w.Write(&Data{
					Address:   Address(rb),
					Timestamp: Timestamp{time.Now()},
					Data:      Buffer(b),
				})

				p.ledbat.AddSent(ChunkSize)
				p.AddSentChunk()
				c.unackedBins.Set(rb)
				c.sentBinHistory.Push(rb)
			}
		}

		c.swarm.Unlock()
		c.Unlock()
		return true
	})

	// only send pings opportunistically with other messages
	if w.Dirty() {
		if nonce, ok := p.TrackPingRTT(); ok {
			w.Write(&Ping{Nonce{nonce}})
		}
	}

	if err := w.Flush(); err != nil {
		log.Println(err)
	}
}

func (s *Scheduler) channelAddedBins(c *channel) (bins []binmap.Bin) {
	for {
		b := c.addedBins.FindFilled()
		if b.IsNone() {
			break
		}
		b = c.swarm.loadedBins.Cover(b)
		if b.IsAll() {
			break
		}
		c.addedBins.Reset(b)

		bins = append(bins, b)
	}
	return
}

func (s *Scheduler) peerRequestCapacity(p *Peer) int {
	p.ledbat.DigestDelaySamples()

	planForDuration := p.ledbat.RTTMean()
	// regardless of how fast our peer is sending us data we're not going to
	// send another request for at least minWriteInterval...
	// TODO: this is repeated below in scheduler, maybe fix that...
	if planForDuration < minWriteInterval {
		planForDuration = minWriteInterval
	}
	planForDuration *= 4
	if planForDuration > time.Second {
		planForDuration = time.Second
	}

	capacity := 1
	if p.ChunkIntervalMean() != 0 {
		capacity = int(planForDuration / p.ChunkIntervalMean())
	}
	capacity -= p.OutstandingChunks()
	if capacity < 1 {
		capacity = 1
	}

	// if !p.ledbat.Debug() {
	// 	log.Println(
	// 		"\np.ledbat.RTTMean()", p.ledbat.RTTMean(),
	// 		"\nminWriteInterval", minWriteInterval,
	// 		"\np.ChunkIntervalMean()", p.ChunkIntervalMean(),
	// 		"\nplanForDuration", planForDuration,
	// 		"\np.OutstandingChunks()", p.OutstandingChunks(),
	// 		"\ncapacity", capacity,
	// 	)
	// }

	return capacity
}

// TODO: select more bins than we need...
// TODO: chunk picker?
func (s *Scheduler) requestBins(count int, c *channel) (bins []binmap.Bin, n int) {
	// TODO: lock c.swarm.requestedBins here

	var rc = uint64(count)
	var ab, bb binmap.Bin
Done:
	for rc > 0 {
		if c.swarm.requestedBins.Filled() {
			ab = c.swarm.requestedBins.RootBin().BaseRight() + 2
		} else if c.swarm.requestedBins.Empty() {
			ab = 0
		} else {
			ab = c.swarm.requestedBins.FindEmptyAfter(ab)
		}

		if !c.availableBins.RootBin().Contains(ab) {
			break Done
		}

		bb = c.availableBins.FindFilledAfter(ab)
		if bb.IsNone() {
			break Done
		}

		ab = c.swarm.requestedBins.Cover(ab)
		bb = c.availableBins.Cover(bb)

		if ab.Contains(bb) {
			ab = bb
		} else if !bb.Contains(ab) {
			ab = bb.BaseLeft()
			continue
		}

		for ab.BaseLength() > rc {
			ab = ab.Left()
		}
		rc -= ab.BaseLength()

		// TODO: limits on contiguous chunk lengths to improve source diversity?

		bins = append(bins, ab)
		c.swarm.requestedBins.Set(ab)

		ab = ab.BaseRight() + 2
	}
	return bins, count - int(rc)
}
