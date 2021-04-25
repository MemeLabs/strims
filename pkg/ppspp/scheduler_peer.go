package ppspp

import (
	"errors"
	"log"
	"math"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ma"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"go.uber.org/zap"
)

var _ SwarmScheduler = &peerSwarmScheduler{}
var _ ChannelScheduler = &peerChannelScheduler{}

const (
	schedulerGCInterval         = 5 * time.Second
	schedulerRateUpdateInterval = time.Second
)

func newPeerSwarmScheduler(logger *zap.Logger, s *Swarm) *peerSwarmScheduler {
	return &peerSwarmScheduler{
		logger:          logger,
		swarm:           s,
		streamBits:      binmap.Bin(streamCount(s.options)) - 1,
		peerHaveBinRate: ma.NewSimple(15, time.Second),
		haveBins:        binmap.New(),
		requestBins:     binmap.New(),
		requestStreams:  map[codec.Stream]binmap.Bin{},
		channels:        map[*Peer]*peerChannelScheduler{},

		integrityOverhead: integrity.MaxMessageBytes(
			s.options.Integrity.ProtectionMethod,
			s.options.Integrity.LiveSignatureAlgorithm,
			s.options.Integrity.MerkleHashTreeFunction,
			s.options.ChunksPerSignature,
		),
		chunkSize:  int(s.options.ChunkSize),
		liveWindow: binmap.Bin(s.options.LiveWindow),
	}
}

type peerSwarmScheduler struct {
	logger *zap.Logger
	swarm  *Swarm

	lock           sync.Mutex
	lastGCTime     time.Time
	rateUpdateTime time.Time

	streamBits binmap.Bin
	binTimes   timeSet

	peerHaveBinRate ma.Simple
	peerMaxHaveBin  binmap.Bin

	haveBins   *binmap.Map
	haveBinMax binmap.Bin

	requestBins    *binmap.Map
	requestStreams map[codec.Stream]binmap.Bin
	channels       map[*Peer]*peerChannelScheduler

	integrityOverhead int
	chunkSize         int
	liveWindow        binmap.Bin

	initHack sync.Once
}

func (s *peerSwarmScheduler) Run(t time.Time) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if t.Sub(s.lastGCTime) >= schedulerGCInterval {
		s.lastGCTime = t
		s.gc()
	}

	if t.Sub(s.rateUpdateTime) >= schedulerRateUpdateInterval {
		s.rateUpdateTime = t

		bps := int(s.peerHaveBinRate.RateWithTime(time.Second, t)) * s.chunkSize

		if bps != 0 {
			for _, cs := range s.channels {
				cs.lock.Lock()
				if cs.testSkip {
					cs.test.SetLimit(0)
				} else {
					cs.test.SetLimit(float64(bps / len(s.channels)))
				}
				cs.lock.Unlock()
			}
		}
	}

	// prune channel request times
	// decide which bin ranges we would consider from each peer

	// when the bitrate is low worry less about who we subscribe to

	// balance... tensor things... elastic springy something...
	// downregulate when requests lag and adjust the target to compensate

	// how do we allocate requests

	// replace underperforming peers...
	// collect peers that swarms could do without?
	// handle this in runner based on ingress size?
	// unique but slow peers are also important..?
}

func (s *peerSwarmScheduler) gc() {
	binTimesThreshold := s.haveBinMax - s.liveWindow*2
	s.binTimes.Prune(binTimesThreshold)

	requestTimesThreshold := s.haveBins.FindEmptyAfter(s.haveBinMax - s.liveWindow)
	if requestTimesThreshold.IsNone() {
		requestTimesThreshold = s.haveBins.RootBin().BaseRight()
	}
	for _, cs := range s.channels {
		cs.lock.Lock()
		cs.requestTimes.Prune(requestTimesThreshold)
		cs.lock.Unlock()
	}
}

func (s *peerSwarmScheduler) Consume(c store.Chunk) bool {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.haveBins.Set(c.Bin)
	hb := s.haveBins.Cover(c.Bin)
	if hb.IsAll() {
		hb = s.haveBins.RootBin()
	}
	for _, c := range s.channels {
		c.appendHaveBins(hb)
	}

	if b := c.Bin.BaseRight(); b > s.haveBinMax {
		s.haveBinMax = b
	}

	// for b, end := sc.Bin.BaseLeft(), sc.Bin.BaseRight(); b <= end; b += 2 {
	// 	t := s.streams[uint16(b)&s.streamIndexMask]
	// 	t.chunks = append(t.chunks, b)
	// 	// for _, p := range t.peers {
	// 	//   enqueue to send
	// 	// }
	// }

	// we probably have to enqueue send afer every write-inducing operation...

	return true
}

func (s *peerSwarmScheduler) ChannelScheduler(p *Peer, cw *channelWriter) ChannelScheduler {
	c := &peerChannelScheduler{
		logger:             s.logger.With(logutil.ByteHex("peer", p.id)),
		p:                  p,
		cw:                 cw,
		s:                  s,
		streamHaveLag:      make([]ma.Welford, streamCount(s.swarm.options)),
		haveBins:           binmap.New(),
		cancelBins:         binmap.New(),
		requestBins:        newBinTimeoutQueue(128),
		requestStreams:     map[codec.Stream]binmap.Bin{},
		peerHaveBins:       binmap.New(),
		peerRequestStreams: map[codec.Stream]binmap.Bin{},

		test: qos.NewHLB(math.MaxFloat64),
	}

	s.lock.Lock()
	s.channels[p] = c
	s.lock.Unlock()

	return c
}

func (s *peerSwarmScheduler) CloseChannel(p *Peer) {
	s.lock.Lock()
	cs, ok := s.channels[p]
	delete(s.channels, p)
	s.lock.Unlock()

	if !ok {
		return
	}

	// remove requested streams

	p.closeChannel(cs)
}

func (s *peerSwarmScheduler) binStream(b binmap.Bin) codec.Stream {
	return codec.Stream(b & s.streamBits)
}

type peerChannelScheduler struct {
	logger *zap.Logger
	p      *Peer
	cw     *channelWriter
	s      *peerSwarmScheduler

	r PeerWriterQueueTicket

	lock sync.Mutex

	choked        bool
	streamHaveLag []ma.Welford
	requestTimes  timeSet
	dataRtt       ma.Welford

	haveBins       *binmap.Map      // bins to send HAVEs for
	cancelBins     *binmap.Map      // bins to send CANCELs for
	requestBins    *binTimeoutQueue // bins recently requested from the peer
	requestStreams map[codec.Stream]binmap.Bin

	requestsAdded      uint32
	peerLiveWindow     binmap.Bin
	peerMaxHaveBin     binmap.Bin
	peerHaveBins       *binmap.Map // bins the peer claims to have
	peerRequestStreams map[codec.Stream]binmap.Bin
	peerOpenStreams    []codec.StreamAddress
	peerCloseStreams   []codec.Stream

	test     *qos.HLB
	testSkip bool
}

func (c *peerChannelScheduler) selectRequestBins() {

}

func (c *peerChannelScheduler) appendHaveBins(hb binmap.Bin) {
	c.lock.Lock()
	c.haveBins.Set(hb)
	c.lock.Unlock()

	c.p.enqueue(&c.r, c)
}

func (c *peerChannelScheduler) timeOutRequests() {
	var modified bool

	c.s.lock.Lock()
	c.lock.Lock()

	now := iotime.Load()
	for ri := c.requestBins.IterateUntil(now); ri.Next(); {
		for ei := c.s.haveBins.IterateEmptyAt(ri.Bin()); ei.NextBase(); {
			c.s.requestBins.Reset(ei.Value())
			c.cancelBins.Set(ei.Value())
			modified = true
		}
	}

	c.lock.Unlock()
	c.s.lock.Unlock()

	_ = modified
	// if modified {
	// 	c.p.enqueue(&c.r)
	// }
}

func (c *peerChannelScheduler) WriteData(maxBytes int, b binmap.Bin, pri peerPriority) (int, error) {
	// TODO: this should run on a timer
	// c.timeOutRequests()

	if err := c.cw.Resize(maxBytes); err != nil {
		return 0, nil
	}

	for {
		if maxBytes >= int(b.BaseLength())*c.s.chunkSize+c.s.integrityOverhead {
			break
		}

		if b.Base() {
			c.p.pushFrontData(c, b, pri)
			return 0, nil
		}

		c.p.pushFrontData(c, b.Right(), pri)
		b = b.Left()
	}

	c.lock.Lock()
	_, err := c.s.swarm.verifier.WriteIntegrity(b, c.haveBins, c.cw)
	c.lock.Unlock()
	if err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.pushFrontData(c, b, pri)
			return 0, nil
		}
		return 0, err
	}
	if _, err := c.s.swarm.store.WriteData(b, c.cw); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.pushFrontData(c, b, pri)
			return 0, nil
		}
		return 0, err
	}

	return c.flushWrites()
}

func (c *peerChannelScheduler) Write(maxBytes int) (int, error) {
	if err := c.cw.Resize(maxBytes); err != nil {
		return 0, nil
	}

	if err := c.write0(); err != nil {
		return 0, err
	}
	if err := c.write1(); err != nil {
		return 0, err
	}

	return c.flushWrites()
}

func (c *peerChannelScheduler) write0() error {
	c.lock.Lock()
	defer c.lock.Unlock()

	if err := c.writeMapBins(c.haveBins, c.writeHave); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			return nil
		}
		return err
	}

	if err := c.writeMapBins(c.cancelBins, c.writeCancel); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			return nil
		}
		return err
	}

	return nil
}

func (c *peerChannelScheduler) write1() error {
	c.s.lock.Lock()
	c.lock.Lock()

	var min binmap.Bin
	if c.peerMaxHaveBin > c.peerLiveWindow {
		min = c.peerMaxHaveBin - c.peerLiveWindow
	}

	var err error
	it := binmap.NewIntersectionIterator(c.s.requestBins.IterateEmptyAt(c.peerHaveBins.RootBin()), c.peerHaveBins.IterateFilled())
	// EachCandidate:
	for ok := it.NextAfter(min); ok; ok = it.Next() {
		b := it.Value()
		// for {
		// 	if c.test.Check(float64(b.BaseLength() * uint64(c.s.chunkSize))) {
		// 		break
		// 	}
		// 	if b.Base() {
		// 		break EachCandidate
		// 	}
		// 	b = b.BaseLeft()
		// }

		_, err = c.cw.WriteRequest(codec.Request{Address: codec.Address(b)})
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			err = nil
			break
		} else if err != nil {
			break
		}
		c.s.requestBins.Set(it.Value())

		c.s.initHack.Do(func() {
			c.s.swarm.store.SetOffset(it.Value().BaseRight())
		})
	}

	c.s.lock.Unlock()
	c.lock.Unlock()

	return err
}

func (c *peerChannelScheduler) writeMapBins(m *binmap.Map, w func(b binmap.Bin) error) error {
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

func (c *peerChannelScheduler) writeHave(b binmap.Bin) error {
	_, err := c.cw.WriteHave(codec.Have{Address: codec.Address(b)})
	return err
}

func (c *peerChannelScheduler) writeCancel(b binmap.Bin) error {
	_, err := c.cw.WriteCancel(codec.Cancel{Address: codec.Address(b)})
	return err
}

func (c *peerChannelScheduler) flushWrites() (int, error) {
	n := c.cw.Len()
	if err := c.cw.Flush(); err != nil {
		return 0, err
	}
	return n, nil
}

func (c *peerChannelScheduler) HandleHandshake(liveWindow uint32) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.peerLiveWindow = binmap.Bin(liveWindow)
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleAck(b binmap.Bin, delaySample time.Duration) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// ignore these for now?
	return nil
}

var tb, tc uint64

func (c *peerChannelScheduler) HandleData(b binmap.Bin, valid bool) error {
	if !valid {
		// TODO: this should probably use a binmap so we can unset cancelled bins
		// TODO: this needs to account for chunks we receive from streams
		// c.lock.Lock()
		// if _, ok := c.requestTimes.Get(b); !ok {
		// 	return nil
		// }
		// c.lock.Unlock()

		c.s.lock.Lock()
		c.s.requestBins.Reset(b)
		c.s.lock.Unlock()
		return nil
	}

	ntc := atomic.AddUint64(&tc, 1)
	ntb := atomic.AddUint64(&tb, b.BaseLength()*uint64(c.s.chunkSize))
	if ntc%1000 == 0 {
		log.Printf("HandleData bytes: %d chunks: %d", ntb, ntc)
	}

	now := iotime.Load()

	c.p.lock.Lock()
	c.p.receivedBytes.AddWithTime(b.BaseLength()*uint64(c.s.chunkSize), now)
	c.p.lock.Unlock()

	c.lock.Lock()
	if ts, ok := c.requestTimes.Get(b); ok {
		c.dataRtt.Update(float64(now.UnixNano() - ts))
	}
	c.lock.Unlock()
	return nil
}

var za, zb int64

func (c *peerChannelScheduler) HandleHave(b binmap.Bin) error {
	// TODO: reject far future/past bins

	t := iotime.Load()
	ts := t.UnixNano()

	c.s.lock.Lock()
	// tts := time.Now()
	c.s.binTimes.Set(b, ts)

	c.lock.Lock()

	for it := c.peerHaveBins.IterateEmptyAt(b); it.NextBase(); {
		first, _ := c.s.binTimes.Get(it.Value())
		c.streamHaveLag[(it.Value()>>1)&c.s.streamBits].Update(float64(ts - first))

		// TODO: consolidate bins before setting request bins?
		rb, ok := c.s.requestStreams[c.s.binStream(it.Value())]
		if ok && rb <= it.Value() {
			c.s.requestBins.Set(it.Value())
		}
	}

	debug.RunEveryN(100000, func() {
		d := make([]time.Duration, len(c.streamHaveLag))
		for i := range c.streamHaveLag {
			d[i] = time.Duration(c.streamHaveLag[i].Mean())
		}
		log.Printf("times %s", d)
	})

	// nzb := atomic.AddInt64(&zb, int64(time.Since(tts)))
	// nza := atomic.AddInt64(&za, 1)

	br := b.BaseRight()
	if br > c.peerMaxHaveBin {
		c.peerMaxHaveBin = br
	}
	if br > c.s.peerMaxHaveBin {
		if c.s.peerMaxHaveBin != 0 {
			c.s.peerHaveBinRate.AddWithTime(uint64(br-c.s.peerMaxHaveBin)/2, t)
		}
		c.s.peerMaxHaveBin = br
	}

	c.s.lock.Unlock()

	c.peerHaveBins.Set(b)

	c.lock.Unlock()

	// debug.LogfEveryN(1000, "write1 count: %d time: %s avg: %d", nza, time.Duration(nzb), nzb/nza)

	return nil
}

func (c *peerChannelScheduler) HandleRequest(b binmap.Bin) error {
	c.p.pushData(c, b, peerPriorityLow)

	atomic.StoreUint32(&c.requestsAdded, 1)

	return nil
}

func (c *peerChannelScheduler) HandleCancel(b binmap.Bin) error {
	c.p.removeData(c, b, peerPriorityLow)

	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleChoke() error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.choked = true
	// c.s.requestedBins.Reset(c.s.requestedBins.RootBin())
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleUnchoke() error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.choked = false
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandlePing(nonce uint64) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// if time since last ping > threshold enqueue
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandlePong(nonce uint64) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// update rtt
	return nil
}

func (c *peerChannelScheduler) HandleStreamRequest(s codec.Stream, b binmap.Bin) error {
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

func (c *peerChannelScheduler) HandleStreamCancel(s codec.Stream) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// delete enqueued sends in this stream
	delete(c.peerRequestStreams, s)
	c.peerCloseStreams = append(c.peerCloseStreams, s)
	return nil
}

func (c *peerChannelScheduler) HandleStreamOpen(s codec.Stream, b binmap.Bin) error {
	c.s.lock.Lock()
	defer c.s.lock.Unlock()
	c.lock.Lock()
	defer c.lock.Unlock()
	// add to stream set

	/*
		when do we want to allow this?
			* it was requested
			* we have no information?
			* the peer is already the fastest known peer (ie seeds?)
			* this peer already sends us >threshold data?
			* we have no existing subscription for this stream?
	*/

	c.requestStreams[s] = b
	c.s.requestStreams[s] = b

	c.testSkip = true

	return nil
}

func (c *peerChannelScheduler) HandleStreamClose(s codec.Stream) error {
	c.s.lock.Lock()
	defer c.s.lock.Unlock()
	c.lock.Lock()
	defer c.lock.Unlock()

	// remove from stream set
	delete(c.requestStreams, s)
	delete(c.s.requestStreams, s)

	// reset outstanding requests
	// iterate through the bins in s > first empty in swarm bins

	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleMessageEnd() error {
	if atomic.CompareAndSwapUint32(&c.requestsAdded, 1, 0) {
		c.p.enqueueNow(&c.r, c)
	} else {
		c.p.enqueue(&c.r, c)
	}

	return nil
}
