package ppspp

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"go.uber.org/zap"
)

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
	close  sync.Map
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
		rescheduleTicker := time.NewTicker(rescheduleInterval)

		writeInterval := defaultWriteInterval
		writeTicker := time.NewTicker(writeInterval)

		for {
			select {
			case t := <-writeTicker.C:
				r.runPeer(peer, iotime.FromTime(t))
			case <-ctx.Done():
				return
			case <-rescheduleTicker.C:
				newWriteInterval := r.peerWriteInterval(peer)
				if writeInterval != newWriteInterval {
					r.logger.Debug("updating write interval", zap.Duration("duration", newWriteInterval))
					writeTicker.Stop()
					writeTicker = time.NewTicker(newWriteInterval)
					writeInterval = newWriteInterval
				}
			}
		}
	}()
}

// RemovePeer ...
func (r *Scheduler) RemovePeer(peer *Peer) {
	close, ok := r.close.Load(peer)
	if ok {
		close.(context.CancelFunc)()
		r.close.Delete(peer)
	}
}

func (r *Scheduler) peerWriteInterval(peer *Peer) (i time.Duration) {
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

func (r *Scheduler) runPeer(p *Peer, t time.Time) {
	p.Lock()
	defer p.Unlock()

	r.sendPongs(p, t)
	r.sendPeerTimeouts(p, t)
	// r.sendPeerExchange(p, w)
	r.sendPeerData(p, t)
	r.sendPeerPing(p, t)

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

func (r *Scheduler) sendPeerTimeouts(p *Peer, t time.Time) {
	// TODO: separate read and write timeouts
	// TODO: mediate cancel/retry floods from increased latency
	// timeout := time.Now().Add(-p.ledbat.CTO() * 2)
	timeout := t.Add(-2 * time.Second)

	for s, c := range p.channels {
		c.Lock()

		for i := c.sentBinHistory.IterateUntil(timeout); i.Next(); {
			if c.unackedBins.FilledAt(i.Bin()) {
				p.ledbat.AddDataLoss(s.chunkSize(), false)
				c.unackedBins.Reset(i.Bin())
			}
		}

		for i := c.requestedBinHistory.IterateUntil(timeout); i.Next(); {
			if !s.store.FilledAt(i.Bin()) {
				for b := i.Bin().BaseLeft(); b <= i.Bin().BaseRight(); b += 2 {
					if s.store.EmptyAt(b) {
						s.bins.Lock()
						s.bins.Requested.Reset(b)
						s.bins.Unlock()
						p.AddCancelledChunk()
					}
				}

				c.WriteCancel(codec.Cancel{Address: codec.Address(i.Bin())})
			}
		}

		c.Unlock()
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
		for _, a := range c.acks {
			c.WriteAck(a)
		}
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

			c.WriteHave(codec.Have{Address: codec.Address(b)})
		}

		requestBins, n := r.requestBins(requesteCapacity, s, c.channel)
		requesteCapacity -= n

		for _, b := range requestBins {
			c.WriteRequest(codec.Request{Address: codec.Address(b)})
			p.AddRequestedChunks(b.BaseLength())
			c.requestedBinHistory.Push(b, t)
		}

		b := pool.Get(uint16(s.chunkSize()))

		// TODO: rlock s.chunks here
		for p.ledbat.FlightSize() < p.ledbat.CWND() {
			rb := c.requestedBins.FindFilled()
			if rb.IsNone() {
				break
			}
			rb = rb.BaseLeft()
			c.requestedBins.Reset(rb)

			if ok := s.store.ReadBin(rb, b); ok {
				// TODO: avoid writing data until after this?
				c.WriteData(codec.Data{
					Address:   codec.Address(rb),
					Timestamp: codec.Timestamp{Time: t},
					Data:      codec.Buffer(b),
				})

				p.ledbat.AddSent(s.chunkSize())
				c.unackedBins.Set(rb)
				c.sentBinHistory.Push(rb, t)
			}
		}

		pool.Put(b)

		c.Unlock()
	}
}

func (r *Scheduler) sendPongs(p *Peer, t time.Time) {
	for _, c := range p.channels {
		if p := c.dequeuePong(); p != nil {
			c.WritePong(*p)
		}
	}
}

func (r *Scheduler) sendPeerPing(p *Peer, t time.Time) {
	for _, c := range p.channels {
		if c.Dirty() {
			if nonce, ok := p.TrackPingRTT(c.id, t); ok {
				c.WritePing(codec.Ping{Nonce: codec.Nonce{Value: nonce}})
			}
		}
	}
}

func (r *Scheduler) peerRequestCapacity(p *Peer) int {
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
	// log.Println(
	// 	"\np.ledbat.RTTMean()", p.ledbat.RTTMean(),
	// 	"\nminWriteInterval", minWriteInterval,
	// 	"\np.ChunkIntervalMean()", p.ChunkIntervalMean(),
	// 	"\nplanForDuration", planForDuration,
	// 	"\np.OutstandingChunks()", p.OutstandingChunks(),
	// 	"\ncapacity", capacity,
	// )
	// }

	return capacity
}

func (r *Scheduler) requestBins(count int, s *Swarm, c *channel) ([]binmap.Bin, int) {
	s.bins.Lock()
	defer s.bins.Unlock()

	if s.bins.Requested.Empty() {
		bins, n := (&FirstChunkSelector{}).SelectBins(count, s.bins.Available, s.bins.Requested, c.availableBins)
		if len(bins) != 0 {
			s.store.SetOffset(bins[0].BaseLeft() + 2)
		}
		return bins, n
	}
	return (&SequentialChunkSelector{}).SelectBins(count, s.bins.Available, s.bins.Requested, c.availableBins)
}

// ChunkSelector ...
type ChunkSelector interface {
	SelectBins(count int, seen, requested, available *binmap.Map) ([]binmap.Bin, int)
}
