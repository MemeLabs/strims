package encoding

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"go.uber.org/zap"
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
func NewScheduler(ctx context.Context, logger *zap.Logger) (s *Scheduler) {
	s = &Scheduler{
		logger: logger,
	}

	return
}

// Scheduler ...
type Scheduler struct {
	logger *zap.Logger
	// host  *Host
	close sync.Map
}

const (
	defaultWriteInterval = 200 * time.Millisecond
	maxWriteInterval     = time.Second
	minWriteInterval     = 50 * time.Millisecond
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

		for {
			select {
			case t := <-writeTicker.C:
				s.runPeer(peer, iotime.FromTime(t))
			case <-ctx.Done():
				return
			case <-rescheduleTicker.C:
				newWriteInterval := s.peerWriteInterval(peer)
				// log.Println("newWriteInterval", unsafe.Pointer(peer), newWriteInterval)
				if writeInterval != newWriteInterval {
					s.logger.Debug("updating write interval", zap.Duration("duration", newWriteInterval))
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

	// peer.Lock()
	// // spew.Dump(peer)
	// if peer.ledbat.Debug() {
	// 	// if peer.requestedChunkCount > 0 {
	// 	channels := []string{}
	// 	for _, c := range peer.channels {
	// 		c.Lock()
	// 		c.swarm.Lock()

	// 		// channels = append(channels, spew.Sdump([]interface{}{
	// 		// 	"id", c.id,
	// 		// 	// "swarm", c.swarm,
	// 		// 	// "peer",  c.peer,
	// 		// 	// "conn",      c.conn,
	// 		// 	// "addedBins", c.addedBins,
	// 		// 	// "requestedBins", c.requestedBins,
	// 		// 	// "availableBins",       c.availableBins,
	// 		// 	// "unackedBins",         c.unackedBins,
	// 		// 	// "rttSampleBin",        c.rttSampleBin,
	// 		// 	"sentBinHistory.ring", c.sentBinHistory.ring,
	// 		// 	"requestedBinHistory.ring", c.requestedBinHistory.ring,
	// 		// }))

	// 		c.swarm.Unlock()
	// 		c.Unlock()
	// 	}

	// 	log.Println(
	// 		// "peer", unsafe.Pointer(peer),
	// 		"\nrequested", peer.requestedChunkCount,
	// 		"\nreceived", peer.receivedChunkCount,
	// 		"\ncancelled", peer.cancelledChunkCount,
	// 		"\nsent", peer.sentChunkCount,
	// 		"\nacked", peer.ackedChunkCount,
	// 		"\nnewly acked", peer.NewlyAckedCount(),
	// 		"\nlost", peer.sentChunkCount-peer.ackedChunkCount,
	// 		"\nOutstandingChunks", peer.OutstandingChunks(),
	// 		"\nChunkIntervalMean", peer.ChunkIntervalMean(),
	// 		"\nledbat.CWND()", peer.ledbat.CWND(),
	// 		"\nledbat.CTO()", peer.ledbat.CTO(),
	// 		"\nledbat.RTTMean()", peer.ledbat.RTTMean(),
	// 		"\nledbat.FlightSize()", peer.ledbat.FlightSize(),
	// 		"\ncwnd count", peer.ledbat.CWND()/ChunkSize,
	// 		"\nflight count", peer.ledbat.FlightSize()/ChunkSize,
	// 		// "loss", 1-float64(peer.receivedChunkCount)/float64(peer.requestedChunkCount),
	// 		// "ledbat", spew.Sdump(peer.ledbat),
	// 		// "peer", spew.Sdump(peer),
	// 		"\nchannels", channels,
	// 		"\n---",
	// 	)
	// }
	// peer.Unlock()
}

func (s *Scheduler) runPeer(p *Peer, t time.Time) {
	p.Lock()
	defer p.Unlock()

	s.sendPeerTimeouts(p, t)
	// s.sendPeerExchange(p, w)
	s.sendPeerData(p, t)
	s.sendPeerPing(p, t)

	// if err := w.Flush(); err != nil {
	// 	log.Println(err)
	// }

	// TODO: only flush/push the last
	for _, c := range p.channels {
		if err := c.w.Flush(); err != nil {
			c.Close()
		}
	}
}

func (s *Scheduler) sendPeerTimeouts(p *Peer, t time.Time) {
	// TODO: separate read and write timeouts
	// TODO: mediate cancel/retry floods from increased latency
	// timeout := time.Now().Add(-p.ledbat.CTO() * 2)
	timeout := t.Add(-2 * time.Second)

	for _, c := range p.channels {
		c.Lock()
		c.swarm.Lock()

		for i := c.sentBinHistory.IterateUntil(timeout); i.Next(); {
			if c.unackedBins.FilledAt(i.Bin()) {
				c.peer.ledbat.AddDataLoss(ChunkSize, false)
				c.unackedBins.Reset(i.Bin())
			}
		}

		for i := c.requestedBinHistory.IterateUntil(timeout); i.Next(); {
			if !c.swarm.chunks.bins.FilledAt(i.Bin()) {
				for b := i.Bin().BaseLeft(); b <= i.Bin().BaseRight(); b += 2 {
					if c.swarm.chunks.bins.EmptyAt(b) {
						c.swarm.requestedBins.Reset(b)
						c.peer.AddCancelledChunk()
					}
				}

				c.w.WriteCancel(Cancel{Address(i.Bin())})
			}
		}

		c.swarm.Unlock()
		c.Unlock()
	}
}

func (s *Scheduler) sendPeerData(p *Peer, t time.Time) {
	requesteCapacity := s.peerRequestCapacity(p)
	for _, c := range p.channels {
		c.Lock()

		if c.choked {
			c.Unlock()
			continue
		}

		c.swarm.Lock()
		// TODO: avoid holding swarm lock during io...

		// TODO: compress ACKs like HAVEs... min(delay sample)
		for _, a := range c.acks {
			c.w.WriteAck(a)
		}
		c.acks = c.acks[:0]

		// TODO: move to channel and execute under addedBinsLock
		// TODO: move insert added bins to channel to resolve Swarm WriteChunk lock inconsistency
		for {
			b := c.addedBins.FindFilled()
			if b.IsNone() {
				break
			}
			b = c.swarm.chunks.bins.Cover(b)
			if b.IsAll() {
				break
			}
			c.addedBins.Reset(b)

			c.w.WriteHave(Have{Address(b)})
		}

		requestBins, n := s.requestBins(requesteCapacity, c)
		requesteCapacity -= n

		for _, b := range requestBins {
			c.w.WriteRequest(Request{Address(b)})
			p.AddRequestedChunks(b.BaseLength())
			c.requestedBinHistory.Push(b, t)
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
				// TODO: avoid writing data until after this?
				c.w.WriteData(Data{
					Address:   Address(rb),
					Timestamp: Timestamp{t},
					Data:      Buffer(b),
				})

				p.ledbat.AddSent(ChunkSize)
				p.AddSentChunk()
				c.unackedBins.Set(rb)
				c.sentBinHistory.Push(rb, t)
			}
		}

		c.swarm.Unlock()
		c.Unlock()
	}
}

func (s *Scheduler) sendPeerPing(p *Peer, t time.Time) {
	for _, c := range p.channels {
		c.Lock()

		if nonce, ok := p.TrackPingRTT(t); ok {
			c.w.WritePing(Ping{Nonce{nonce}})
		}

		c.Unlock()
	}

	// // only send pings opportunistically with other messages
	// if w.Dirty() {
	// 	if nonce, ok := p.TrackPingRTT(); ok {
	// 		w.Write(&Ping{Nonce{nonce}})
	// 	}
	// }
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
// TODO: chunk picker interface
func (s *Scheduler) requestBins(count int, c *channel) (bins []binmap.Bin, n int) {
	// TODO: lock c.swarm.requestedBins here

	if c.swarm.requestedBins.Empty() {
		return s.requestFirstBins(count, c)
	}

	var rc = uint64(count)
	var ab, bb binmap.Bin
Done:
	for rc > 0 {
		if c.swarm.requestedBins.Filled() {
			ab = c.swarm.requestedBins.RootBin().BaseRight() + 2
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

		// TODO: limit contiguous chunk lengths to improve source diversity?

		bins = append(bins, ab)
		c.swarm.requestedBins.Set(ab)

		ab = ab.BaseRight() + 2
	}

	n = count - int(rc)
	return
}

func (s *Scheduler) requestFirstBins(count int, c *channel) (bins []binmap.Bin, n int) {
	// TODO: select some range of bins near the tail of the peer's available
	// set... maybe try to pick the start of the last chunkstream segment?

	if c.availableBins.Empty() {
		return
	}

	// find the last available bin from this peer
	var ab binmap.Bin
	nab := ab
	for {
		nab = c.availableBins.FindFilledAfter(nab)
		if nab.IsNone() || nab.IsAll() || !c.availableBins.RootBin().Contains(nab) {
			break
		}
		ab = nab.BaseRight()

		nab = c.availableBins.Cover(nab).BaseRight()
		if nab.IsNone() || nab.IsAll() || !c.availableBins.RootBin().Contains(nab) {
			break
		}
		ab = nab
		nab += 2
	}

	bins = append(bins, ab)
	c.swarm.requestedBins.Set(ab)

	// TODO: hax...
	c.swarm.chunks.next = ab + 2

	// fill from 0 to ab so the first empty bin is ab + 2
	for ab > 0 {
		ab = c.swarm.requestedBins.Cover(ab - 2)
		c.swarm.requestedBins.Set(ab)
		ab = ab.BaseLeft()
	}
	return
}
