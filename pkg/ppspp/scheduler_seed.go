package ppspp

import (
	"errors"
	"log"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"go.uber.org/zap"
)

var _ swarmScheduler = &seedSwarmScheduler{}
var _ channelScheduler = &seedChannelScheduler{}

func newSeedSwarmScheduler(logger *zap.Logger, s *Swarm) *seedSwarmScheduler {
	return &seedSwarmScheduler{
		logger:     logger,
		swarm:      s,
		streamBits: binmap.Bin(s.options.StreamCount) - 1,
		haveBins:   binmap.New(),
		channels:   map[peerThing]*seedChannelScheduler{},

		integrityOverhead: s.options.IntegrityVerifierOptions().MaxMessageBytes(),
		chunkSize:         int(s.options.ChunkSize),
	}
}

type seedSwarmScheduler struct {
	logger *zap.Logger
	swarm  *Swarm

	lock       sync.Mutex
	streamBits binmap.Bin

	haveBins   *binmap.Map
	haveBinMax binmap.Bin

	channels map[peerThing]*seedChannelScheduler

	integrityOverhead int
	chunkSize         int

	initHack int32
}

func (s *seedSwarmScheduler) Run(c timeutil.Time) {
	// we want to send as many copies of the stream as bandwidth allows
	// but we don't know how much bandwidth is available
	// and we have no way to measure it...

	// we want to split streams proportionally based on peer bandwidth
	// but we can't measure that either
	// should we collect acks?
	// does ledbat make sense here?

	// we want to make sure that every piece is delivered to all of our peers
	// but we could be surrounded by malicious peers

	// fraudulent haves are an open problem... maybe we deal with this later
	// if we assume haves are honest -_-
	// we can measure the "echo" for stream chunks we've sent
	// ie measure the mean time to receive haves from peers we didn't send the stream bin to
	// this is vulnerable to the same dishonest have bin shit that can fuck up stream subs
	// ...so we're already assuming this is trustworthy
	// it would be nice if there was a way to verify o.O

	// ok...
	// we can reuse the requestTime/streams stats thing from peer for this probably
	// maybe maintain per-channel stream stats?

	// when more than one peer get the same seed stream how do we parse the results?
	// can we a/b test this?
	// is this valid since there's no guarantee that the peers connected to us are a representative sample of the swarm?
}

func (s *seedSwarmScheduler) Consume(c store.Chunk) {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.haveBins.Set(c.Bin)
	hb := s.haveBins.Cover(c.Bin)
	if hb.IsAll() {
		hb = s.haveBins.RootBin()
	}
	for _, cs := range s.channels {
		cs.appendHaveBins(hb, c.Bin)
	}
}

func (s *seedSwarmScheduler) ChannelScheduler(p peerThing, cw channelWriterThing) channelScheduler {
	s.lock.Lock()
	defer s.lock.Unlock()

	c := &seedChannelScheduler{
		logger:             s.logger.With(logutil.ByteHex("peer", p.ID())),
		p:                  p,
		cw:                 cw,
		s:                  s,
		haveBins:           s.haveBins.Clone(),
		peerHaveBins:       binmap.New(),
		peerRequestStreams: map[codec.Stream]binmap.Bin{},
	}

	// HAX
	initHack := codec.Stream(atomic.AddInt32(&s.initHack, 1)) - 1
	sc := codec.Stream(c.s.swarm.options.StreamCount)
	k := (sc + 2) / 3
	for i := codec.Stream(0); i < sc; i++ {
		if i/k == initHack {
			c.peerOpenStreams = append(c.peerOpenStreams, codec.StreamAddress{
				Stream:  i,
				Address: 0,
			})
			c.peerRequestStreams[i] = 0

			for it := c.s.haveBins.IterateFilled(); it.NextBase(); {
				if c.s.binStream(it.Value()) == i {
					c.p.PushData(c, it.Value(), timeutil.EpochTime, peerPriorityHigh)
				}
			}
		}
	}

	s.channels[p] = c

	return c
}

func (s *seedSwarmScheduler) CloseChannel(p peerThing) {
	s.lock.Lock()
	cs, ok := s.channels[p]
	delete(s.channels, p)
	s.lock.Unlock()

	if !ok {
		return
	}

	// remove requested streams

	p.CloseChannel(cs)
}

func (s *seedSwarmScheduler) binStream(b binmap.Bin) codec.Stream {
	return codec.Stream(b >> 1 & s.streamBits)
}

type seedChannelScheduler struct {
	logger *zap.Logger
	p      peerThing
	cw     channelWriterThing
	s      *seedSwarmScheduler

	peerWriterQueueTicket

	lock       sync.Mutex
	liveWindow uint32
	choked     bool

	haveBins *binmap.Map // bins to send HAVEs for

	requestsAdded      uint32
	peerHaveBins       *binmap.Map // bins the seed claims to have
	peerRequestStreams map[codec.Stream]binmap.Bin
	peerOpenStreams    []codec.StreamAddress
	peerCloseStreams   []codec.Stream
}

func (c *seedChannelScheduler) selectRequestBins() {

}

func (c *seedChannelScheduler) appendHaveBins(hb, b binmap.Bin) {
	c.lock.Lock()
	c.haveBins.Set(hb)
	c.lock.Unlock()

	// TODO: consolidate ds bins for contiguous streams
	l := b.BaseLeft()
	r := b.BaseRight()
	for bl := l; bl <= r; bl += 2 {
		bs := c.s.binStream(bl)
		rb, ok := c.peerRequestStreams[bs]
		if ok && rb <= bl {
			c.p.PushData(c, bl, timeutil.EpochTime, peerPriorityHigh)
		}
	}

	c.p.Enqueue(c)
}

func (c *seedChannelScheduler) WriteHandshake() error {
	if _, err := c.cw.WriteHandshake(newHandshake(c.s.swarm)); err != nil {
		return err
	}

	if _, err := c.cw.WriteChoke(codec.Choke{}); err != nil {
		return err
	}

	if err := c.writeMapBins(c.haveBins, c.writeHave); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			return nil
		}
		return err
	}

	return nil
}

func (c *seedChannelScheduler) WriteData(maxBytes int, b binmap.Bin, t timeutil.Time, pri peerPriority) (int, error) {
	if err := c.cw.Resize(maxBytes); err != nil {
		c.p.PushFrontData(c, b, t, pri)
		return 0, nil
	}

	for {
		if maxBytes >= int(b.BaseLength())*c.s.chunkSize+c.s.integrityOverhead {
			break
		}

		if b.IsBase() {
			c.p.PushFrontData(c, b, t, pri)
			return 0, nil
		}

		c.p.PushFrontData(c, b.Right(), t, pri)
		b = b.Left()
	}

	if binmap.Bin(22).Contains(b) || b.Contains(22) {
		log.Println(">>> sent data", b)
	}

	c.lock.Lock()
	_, err := c.s.swarm.verifier.WriteIntegrity(b, c.peerHaveBins, c.cw)
	c.lock.Unlock()
	if err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.PushFrontData(c, b, t, pri)
			return 0, nil
		}
		c.logger.Debug(
			"error writing integrity",
			zap.Uint64("bin", uint64(b)),
			zap.Stringer("priority", pri),
			zap.Uint16("stream", uint16(c.s.binStream(b))),
			zap.Error(err),
		)
		return 0, err
	}
	if _, err := c.s.swarm.store.WriteData(b, t, c.cw); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.PushFrontData(c, b, t, pri)
			return 0, nil
		}
		c.logger.Debug(
			"error writing data",
			zap.Uint64("bin", uint64(b)),
			zap.Stringer("priority", pri),
			zap.Uint16("stream", uint16(c.s.binStream(b))),
			zap.Error(err),
		)
		return 0, err
	}

	// TODO: enable optionally?
	c.lock.Lock()
	c.peerHaveBins.Set(b)
	c.lock.Unlock()

	return c.flushWrites()
}

func (c *seedChannelScheduler) Write(maxBytes int) (int, error) {
	if err := c.cw.Resize(maxBytes); err != nil {
		return 0, nil
	}

	if err := c.write0(); err != nil {
		return 0, err
	}

	return c.flushWrites()
}

func (c *seedChannelScheduler) write0() error {
	c.lock.Lock()
	defer c.lock.Unlock()

	if err := c.writeMapBins(c.haveBins, c.writeHave); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			return nil
		}
		return err
	}

	if len(c.peerOpenStreams) != 0 {
		for _, s := range c.peerOpenStreams {
			if _, err := c.cw.WriteStreamOpen(codec.StreamOpen{StreamAddress: s}); err != nil {
				if errors.Is(err, codec.ErrNotEnoughSpace) {
					return nil
				}
				return err
			}
		}
		c.peerOpenStreams = nil
	}

	if len(c.peerCloseStreams) != 0 {
		for _, s := range c.peerCloseStreams {
			if _, err := c.cw.WriteStreamClose(codec.StreamClose{Stream: s}); err != nil {
				if errors.Is(err, codec.ErrNotEnoughSpace) {
					return nil
				}
				return err
			}
		}
		c.peerCloseStreams = nil
	}

	return nil
}

func (c *seedChannelScheduler) writeMapBins(m *binmap.Map, w func(b binmap.Bin) error) error {
	if m.Empty() {
		return nil
	}

	for it := m.IterateFilled(); it.Next(); {
		if err := w(it.Value()); err != nil {
			m.ResetBefore(it.Value())
			return err
		}
	}
	m.Reset(m.RootBin())

	return nil
}

func (c *seedChannelScheduler) writeHave(b binmap.Bin) error {
	_, err := c.cw.WriteHave(codec.Have{Address: codec.Address(b)})
	return err
}

func (c *seedChannelScheduler) flushWrites() (int, error) {
	n := c.cw.Len()
	if err := c.cw.Flush(); err != nil {
		return 0, err
	}
	return n, nil
}

func (c *seedChannelScheduler) HandleHandshake(liveWindow uint32) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.liveWindow = liveWindow
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandleAck(b binmap.Bin, delaySample time.Duration) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// ignore these for now?
	return nil
}

func (c *seedChannelScheduler) HandleData(b binmap.Bin, t timeutil.Time, valid bool) error {
	return nil
}

func (c *seedChannelScheduler) HandleHave(b binmap.Bin) error {
	c.lock.Lock()
	c.peerHaveBins.Set(b)
	c.lock.Unlock()

	return nil
}

func (c *seedChannelScheduler) HandleRequest(b binmap.Bin, t timeutil.Time) error {
	c.p.PushData(c, b, t, peerPriorityLow)

	atomic.StoreUint32(&c.requestsAdded, 1)

	return nil
}

func (c *seedChannelScheduler) HandleCancel(b binmap.Bin) error {
	c.p.RemoveData(c, b, peerPriorityLow)
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandleChoke() error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.choked = true
	// c.s.requestedBins.Reset(c.s.requestedBins.RootBin())
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandleUnchoke() error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.choked = false
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandlePing(nonce uint64) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// if time since last ping > threshold enqueue
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandlePong(nonce uint64) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// update rtt
	return nil
}

func (c *seedChannelScheduler) HandleStreamRequest(s codec.Stream, b binmap.Bin) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// add to requested streams map
	c.peerRequestStreams[s] = b
	c.peerOpenStreams = append(c.peerOpenStreams, codec.StreamAddress{
		Stream:  s,
		Address: codec.Address(b),
	})

	// add bins in stream s >= b to send queue
	return nil
}

func (c *seedChannelScheduler) HandleStreamCancel(s codec.Stream) error {
	log.Printf("wut - unsubbed from seed stream %d", s)
	c.lock.Lock()
	defer c.lock.Unlock()
	// delete enqueued sends in this stream
	delete(c.peerRequestStreams, s)
	// c.peerCloseStreams = append(c.peerCloseStreams, s)
	return nil
}

func (c *seedChannelScheduler) HandleStreamOpen(s codec.Stream, b binmap.Bin) error {
	return nil
}

func (c *seedChannelScheduler) HandleStreamClose(s codec.Stream) error {
	return nil
}

// deprecated?
func (c *seedChannelScheduler) HandleMessageEnd() error {
	/*
		if the send queue has bins enqueue to run immediately
		if we have a pong to send enqueue to run immediately
		if we have added bins to announce enqueue
	*/

	// if c.seedRequestBins.Len() != 0 {
	// 	// enqueue now
	// }

	if atomic.CompareAndSwapUint32(&c.requestsAdded, 1, 0) {
		c.p.EnqueueNow(c)
	} else {
		c.p.Enqueue(c)
	}

	return nil
}
