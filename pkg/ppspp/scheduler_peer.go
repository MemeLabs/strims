package ppspp

import (
	"errors"
	"log"
	"math"
	"math/bits"
	"math/rand"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/etcp"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/stats"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"go.uber.org/zap"
)

var _ SwarmScheduler = &peerSwarmScheduler{}
var _ ChannelScheduler = &peerChannelScheduler{}

const (
	schedulerGCInterval          = 5 * time.Second
	schedulerRateUpdateInterval  = 1 * time.Second
	schedulerStreamCheckInterval = 3 * time.Second

	timeGranularity   = int64(time.Millisecond)
	minRequestTimeout = 100 * time.Millisecond
	maxRequestTimeout = 500 * time.Millisecond
)

var debugHackCounter int32

func newPeerSwarmScheduler(logger *zap.Logger, s *Swarm) *peerSwarmScheduler {
	// HAX
	s.store.SetOffset(0)

	sc := streamCount(s.options)
	return &peerSwarmScheduler{
		logger:          logger,
		swarm:           s,
		streamCount:     sc,
		streamLayer:     bits.TrailingZeros16(uint16(sc)),
		streamBits:      binmap.Bin(sc) - 1,
		streamFills:     make([]int, sc*4),
		peerHaveBinRate: stats.NewSMA(15, time.Second),
		haveBins:        binmap.New(),
		peerHaveBins:    binmap.New(),
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
		debugHack:  atomic.AddInt32(&debugHackCounter, 1) == 2,

		handledData: binmap.New(),

		// HAX
		lastGCTime:      time.Now().Add(time.Duration(rand.Intn(5000)) * time.Millisecond),
		rateUpdateTime:  time.Now().Add(time.Duration(rand.Intn(1000)) * time.Millisecond),
		streamCheckTime: time.Now().Add(time.Duration(rand.Intn(3000)) * time.Millisecond),
	}
}

type peerSwarmScheduler struct {
	logger *zap.Logger
	swarm  *Swarm

	lock           sync.Mutex
	lastGCTime     time.Time
	rateUpdateTime time.Time

	streamCount codec.Stream
	streamLayer int
	streamBits  binmap.Bin
	streamFills []int
	binTimes    timeSet

	peerHaveBinRate stats.SMA
	peerMaxHaveBin  binmap.Bin

	haveBins   *binmap.Map
	haveBinMax binmap.Bin

	peerHaveBins    *binmap.Map
	streamCheckTime time.Time

	requestBins    *binmap.Map
	requestStreams map[codec.Stream]binmap.Bin
	channels       map[*Peer]*peerChannelScheduler

	integrityOverhead int
	chunkSize         int
	liveWindow        binmap.Bin

	initHack  sync.Once
	debugHack bool

	waste       uint64
	handledData *binmap.Map
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
				// if cs.testSkip {
				// 	cs.test.SetLimit(0)
				// } else {
				// 	cs.test.SetLimit(float64(bps / (len(s.channels) - 1)))
				// }
				// cs.test.SetLimit(float64(bps / (len(s.channels) - 1)))
				cs.lock.Unlock()
			}
		}
	}

	if t.Sub(s.streamCheckTime) >= schedulerStreamCheckInterval {
		s.streamCheckTime = t

		s.checkStreams()
	}

	for _, cs := range s.channels {
		cs.lock.Lock()
		cs.timeOutRequests()
		cs.lock.Unlock()
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

func (s *peerSwarmScheduler) checkStreams() {
	for _, cs := range s.channels {
		cs.lock.Lock()
	}

	if s.debugHack {
		// lags := []float64{}
		// for _, cs := range s.channels {
		// 	lags = lags[:0]
		// 	for _, l := range cs.streamHaveLag {
		// 		lags = append(lags, l.Mean())
		// 	}
		// 	log.Printf("%4.0f", lags)
		// }

		now := iotime.Load()

		for _, cs := range s.channels {
			s.logger.Debug(
				"data",
				zap.Float64("rtt mean", cs.dataRTT.Mean()),
				zap.Float64("rtt stddev", cs.dataRTT.StdDev()),
				zap.Uint64("chunk rate", cs.dataChunks.SampleRateWithTime(time.Second, now)),
				zap.Uint64("flight size", cs.flightSize),
				zap.Int("cwnd", cs.etcp.CWND()),
			)
		}
	}

	// if s.debugHack {
	// 	// it := binmap.NewIntersectionIterator(s.haveBins.IterateEmpty(), s.requestBins.IterateFilled())
	// 	// it := s.haveBins.IterateEmpty()
	// 	it := s.swarm.store.Bins().IterateEmpty()
	// 	var wut []binmap.Bin
	// 	for it.NextBase() {
	// 		if _, ok := s.requestStreams[s.binStream(it.Value())]; ok {
	// 			wut = append(wut, it.Value())
	// 		}
	// 	}
	// 	if wut != nil {
	// 		log.Printf("wut - %d", wut)
	// 	}
	// }

	{
		lags := make([]stats.Welford, len(s.channels))
		for i := codec.Stream(0); i < s.streamCount; i++ {
			if _, ok := s.requestStreams[i]; ok {
				continue
			}

			// for i := codec.Stream(4); i == 4; i++ {
			lags = lags[:0]
			for _, cs := range s.channels {
				if cs.streamHaveLag[i].Count() > 0 {
					lags = append(lags, cs.streamHaveLag[i])
				}
			}
			if len(lags) == 0 {
				continue
			}
			// log.Println(spew.Sdump(lags))

			lag := stats.WelfordMerge(lags...)

			var minCS *peerChannelScheduler
			var minLag = math.Inf(1)

			for _, cs := range s.channels {
				l := cs.streamHaveLag[i]

				if l.Count() > 0 {
					m := l.Mean()
					p := stats.TDistribution(stats.WelchTTest(l, lag), stats.WelchSatterthwaite(l, lag))

					// consider data rtt and cwnd?

					if l.Count() > 25 && p <= 0.05 && m < minLag {
						minCS = cs
						minLag = m
					}

					// s.logger.Debug(
					// 	"channel stream perf",
					// 	zap.Float64s("count", []float64{l.Count(), lag.Count()}),
					// 	zap.Float64s("mean", []float64{m, lag.Mean()}),
					// 	// zap.Float64s("var", []float64{l.Variance(), lag.Variance()}),
					// 	// zap.Float64("t", stats.WelchTTest(l, lag)),
					// 	// zap.Float64("v", stats.WelchSatterthwaite(l, lag)),
					// 	zap.Float64("p", p),
					// )
				}
			}
			// fmt.Print("\n")

			if minCS != nil {
				s.setHasStream(i)

				b := s.requestBins.FindLastFilled() + 2

				minCS.logger.Debug(
					"subscribed to stream",
					zap.Uint64("stream", uint64(i)),
					zap.Uint64("bin", uint64(b)),
				)

				s.requestStreams[i] = b
				minCS.addStreamRequest(i, b)

				// fill stream bins from last requested bin until last seen bin

				so := streamBinOffset(s.streamCount)
				min := b/so*so + streamBinOffset(i)
				if min < b {
					min += so
				}
				for b := min; b <= s.peerMaxHaveBin; b += so {
					s.requestBins.Set(b)
				}
			}
		}
	}

	for _, cs := range s.channels {
		// log.Printf("%2d", cs.streamFirstCount)
		for i := range cs.streamFirstCount {
			cs.streamFirstCount[i] = 0
		}
	}

	for _, cs := range s.channels {
		for i := range cs.streamHaveLag {
			cs.streamHaveLag[i].Reset()
		}
		cs.lock.Unlock()
	}
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

func (s *peerSwarmScheduler) Consume(c store.Chunk) {
	s.lock.Lock()
	defer s.lock.Unlock()

	// TODO: map[stream]map[*peerSwarmScheduler]bin...
	for it := s.haveBins.IterateEmptyAt(c.Bin); it.NextBase(); {
		for p, cs := range s.channels {
			cs.lock.Lock()
			if sb, ok := cs.peerRequestStreams[s.binStream(it.Value())]; ok && sb <= it.Value() {
				p.pushData(cs, it.Value(), peerPriorityHigh)
				// p.enqueue(&cs.r, cs)
			}
			cs.lock.Unlock()
		}
	}

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
}

func (s *peerSwarmScheduler) ChannelScheduler(p *Peer, cw *channelWriter) ChannelScheduler {
	s.lock.Lock()
	defer s.lock.Unlock()

	c := &peerChannelScheduler{
		logger:             s.logger.With(logutil.ByteHex("peer", p.id)),
		p:                  p,
		cw:                 cw,
		s:                  s,
		streamHaveLag:      make([]stats.Welford, s.streamCount),
		streamFirstCount:   make([]uint64, s.streamCount),
		dataRTTMean:        stats.NewEMA(0.125),
		dataRTTVar:         stats.NewEMA(0.25),
		dataChunks:         stats.NewSMA(60, 100*time.Millisecond),
		haveBins:           s.haveBins.Clone(),
		cancelBins:         binmap.New(),
		requestStreams:     map[codec.Stream]binmap.Bin{},
		peerHaveBins:       binmap.New(),
		peerRequestStreams: map[codec.Stream]binmap.Bin{},

		test: qos.NewHLB(math.MaxFloat64),

		etcp: etcp.NewControl(),

		// written:     binmap.New(),
		// cancelled:   binmap.New(),
		// requested:   binmap.New(),
		// requestSent: binmap.New(),
		// cancelSent:  binmap.New(),
		// pushedData:  binmap.New(),
	}
	s.channels[p] = c

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
	return codec.Stream(b >> 1 & s.streamBits)
}

func (s *peerSwarmScheduler) binHasStream(b binmap.Bin) bool {
	return s.streamFills[b&s.streamBits] > 0
}

func (s *peerSwarmScheduler) setHasStream(v codec.Stream) {
	b := streamBinOffset(v)
	if s.streamFills[b] > 0 {
		return
	}

	for i := 0; i < s.streamLayer; i++ {
		s.streamFills[b]++
		b = b.Parent()
	}
}

func (s *peerSwarmScheduler) unsetHasStream(v codec.Stream) {
	b := streamBinOffset(v)
	if s.streamFills[b] == 0 {
		return
	}

	for i := 0; i < s.streamLayer; i++ {
		s.streamFills[b]--
		b = b.Parent()
	}
}

type peerChannelScheduler struct {
	logger *zap.Logger
	p      *Peer
	cw     *channelWriter
	s      *peerSwarmScheduler

	r PeerWriterQueueTicket

	lock sync.Mutex

	choked           bool
	streamHaveLag    []stats.Welford
	streamFirstCount []uint64
	requestTimes     timeSet
	dataRTT          stats.Welford
	dataRTTMean      stats.EMA
	dataRTTVar       stats.EMA
	dataChunks       stats.SMA

	haveBins       *binmap.Map // bins to send HAVEs for
	cancelBins     *binmap.Map // bins to send CANCELs for
	requestBins    binQueue    // bins recently requested from the peer
	streamBins     binQueue
	requestStreams map[codec.Stream]binmap.Bin
	extraMessages  []codec.Message

	requestsAdded      uint32
	peerLiveWindow     binmap.Bin
	peerMaxHaveBin     binmap.Bin
	peerHaveBins       *binmap.Map // bins the peer claims to have
	peerRequestStreams map[codec.Stream]binmap.Bin
	peerOpenStreams    []codec.StreamAddress
	peerCloseStreams   []codec.Stream

	test *qos.HLB
	// testSkip bool

	etcp       *etcp.Control
	flightSize uint64

	waste uint64
	// written     *binmap.Map
	// cancelled   *binmap.Map
	// requested   *binmap.Map
	// requestSent *binmap.Map
	// cancelSent  *binmap.Map
	// pushedData  *binmap.Map
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
	var n uint64
	var bins []uint64

	for ri := c.requestBins.IterateLessThan(iotime.Load().UnixNano()); ri.Next(); {
		for ei := c.s.haveBins.IterateEmptyAt(ri.Value()); ei.NextBase(); {
			// if (binmap.Bin(22).Contains(ei.Value()) || ei.Value().Contains(22)) && c.s.debugHack {
			// 	log.Println(">>> timed out request")
			// }

			c.s.requestBins.Reset(ei.Value())
			c.cancelBins.Set(ei.Value())
			n += ei.Value().BaseLength()
			modified = true
			bins = append(bins, uint64(ei.Value()))
		}
	}

	if modified {
		c.logger.Debug(
			"timed out requests",
			zap.Uint64("chunks", n),
			zap.Uint64s("bins", bins),
		)

		// c.logger.Debug("timed out requests...", zap.Uint64("count", n))
		c.etcp.OnDataLoss()
		if c.flightSize < n {
			c.flightSize = 0
		} else {
			c.flightSize -= n
		}

		c.p.enqueue(&c.r, c)
	}
}

func (c *peerChannelScheduler) WriteData(maxBytes int, b binmap.Bin, pri peerPriority) (int, error) {
	// TODO: this should run on a timer
	// c.timeOutRequests()

	if err := c.cw.Resize(maxBytes); err != nil {
		c.p.pushFrontData(c, b, pri)
		return 0, nil
	}

	for {
		if maxBytes >= int(b.BaseLength())*c.s.chunkSize+c.s.integrityOverhead {
			break
		}

		if b.IsBase() {
			c.p.pushFrontData(c, b, pri)
			return 0, nil
		}

		c.p.pushFrontData(c, b.Right(), pri)
		b = b.Left()
	}

	c.lock.Lock()
	_, err := c.s.swarm.verifier.WriteIntegrity(b, c.peerHaveBins, c.cw)
	c.lock.Unlock()
	if err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.pushFrontData(c, b, pri)
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
	if _, err := c.s.swarm.store.WriteData(b, c.cw); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.pushFrontData(c, b, pri)
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

	// if !c.written.EmptyAt(b) {
	// 	cancelled := !c.cancelled.EmptyAt(b)
	// 	requested := !c.requested.EmptyAt(b)
	// 	log.Printf("waste - double write cancelled %t requested %t", cancelled, requested)
	// 	// os.Exit(1)
	// }
	// c.written.Set(b)

	// TODO: enable optionally?
	c.peerHaveBins.Set(b)

	// HAX: because we don't send HAVE bins from the seed this doesn't get set
	c.s.lock.Lock()
	c.s.requestBins.Set(b)
	c.s.lock.Unlock()

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

	for i, m := range c.extraMessages {
		var err error
		switch m := m.(type) {
		case *codec.StreamRequest:
			_, err = c.cw.WriteStreamRequest(*m)
		case *codec.StreamCancel:
			_, err = c.cw.WriteStreamCancel(*m)
		case *codec.StreamOpen:
			_, err = c.cw.WriteStreamOpen(*m)
		case *codec.StreamClose:
			_, err = c.cw.WriteStreamClose(*m)
		}

		if err != nil {
			c.pruneExtraMessages(i)
			if errors.Is(err, codec.ErrNotEnoughSpace) {
				return nil
			}
			return err
		}
	}
	c.pruneExtraMessages(len(c.extraMessages))

	return nil
}

func (c *peerChannelScheduler) write1() error {
	c.s.lock.Lock()
	c.lock.Lock()

	var min binmap.Bin
	if c.peerMaxHaveBin > 2*c.peerLiveWindow {
		min = c.peerMaxHaveBin - 2*c.peerLiveWindow
	}
	if m := c.s.haveBinMax - 2*c.s.liveWindow; min > m {
		min = m
	}

	// // HAX
	// if !c.peerHaveBins.Empty() {
	// 	c.s.initHack.Do(func() {
	// 		b := c.peerHaveBins.FindLastFilled()
	// 		c.s.swarm.store.SetOffset(b.BaseRight())
	// 		c.s.requestBins.FillBefore(b.BaseRight() - 2)
	// 	})
	// }

	now := iotime.Load()
	ts := now.UnixNano()
	timeout := now.Add(c.requestTimeout()).UnixNano()
	debug.LogfEveryN(
		100,
		"request timeout %s flightSize: %d cwnd %d",
		c.requestTimeout(),
		c.flightSize,
		c.etcp.CWND(),
	)

	var err error

	n := uint64(c.etcp.CWND()) - c.flightSize
	if n > 0 {
		nl := uint64(bits.Len64(n)) - 1

		it := binmap.NewIntersectionIterator(c.s.requestBins.IterateEmptyAt(c.peerHaveBins.RootBin()), c.peerHaveBins.IterateFilled())
	EachUnrequestedBin:
		for ok := it.NextAfter(min); ok; ok = it.Next() {
			b := it.Value()
			r := b.BaseRight()

			for b <= r {
				if b.Layer() > nl {
					b = b.LayerShift(nl)
				}
				for c.s.binHasStream(b) && !b.IsBase() {
					b = b.Left()
				}

				_, err = c.cw.WriteRequest(codec.Request{Address: codec.Address(b)})
				if err != nil {
					break EachUnrequestedBin
				}

				// c.requestSent.Set(b)
				c.s.requestBins.Set(b)
				c.requestTimes.Set(b, ts)
				c.requestBins.Push(b, timeout)
				c.flightSize += b.BaseLength()

				n = uint64(c.etcp.CWND()) - c.flightSize
				if n == 0 {
					break EachUnrequestedBin
				}
				nl = uint64(bits.Len64(n)) - 1

				b = b.LayerRight()
			}
		}
	}

	c.lock.Unlock()
	c.s.lock.Unlock()

	if errors.Is(err, codec.ErrNotEnoughSpace) {
		return nil
	}
	return err
}

func (c *peerChannelScheduler) requestTimeout() time.Duration {
	rtt := c.dataRTTMean.Value()
	if rtt == 0 {
		return maxRequestTimeout
	}
	return time.Duration(rtt + math.Max(200, 4*c.dataRTTVar.Value())*float64(timeGranularity))

	// if c.dataRTT.Count() == 0 {
	// 	return maxRequestTimeout
	// }

	// timeout := time.Duration(c.dataRTT.Mean() + math.Max(100, 4*c.dataRTT.StdDev())*float64(timeGranularity))
	// if timeout > maxRequestTimeout {
	// 	timeout = maxRequestTimeout
	// } else if timeout < minRequestTimeout {
	// 	timeout = minRequestTimeout
	// }
	// return timeout
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
	// c.cancelSent.Set(b)
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

func (c *peerChannelScheduler) addStreamRequest(s codec.Stream, b binmap.Bin) {
	// c.requestStreams[s] = b

	c.extraMessages = append(c.extraMessages, &codec.StreamRequest{
		StreamAddress: codec.StreamAddress{
			Stream:  s,
			Address: codec.Address(b),
		},
	})
}

func (c *peerChannelScheduler) addStreamCancel(s codec.Stream) {
	delete(c.requestStreams, s)

	c.extraMessages = append(c.extraMessages, &codec.StreamCancel{
		Stream: s,
	})
}

func (c *peerChannelScheduler) addStreamOpen(s codec.Stream, b binmap.Bin) {
	// add to requested streams map
	c.peerRequestStreams[s] = b
	// add bins in stream s >= b to send queue

	c.extraMessages = append(c.extraMessages, &codec.StreamOpen{
		StreamAddress: codec.StreamAddress{
			Stream:  s,
			Address: codec.Address(b),
		},
	})
}

func (c *peerChannelScheduler) addStreamClose(s codec.Stream) {
	// delete enqueued sends in this stream
	delete(c.peerRequestStreams, s)

	c.extraMessages = append(c.extraMessages, &codec.StreamClose{
		Stream: s,
	})
}

func (c *peerChannelScheduler) pruneExtraMessages(i int) {
	n := copy(c.extraMessages, c.extraMessages[i:])
	for i := n; i < len(c.extraMessages); i++ {
		c.extraMessages[i] = nil
	}
	c.extraMessages = c.extraMessages[:n]
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

var tb, tc, td uint64
var waste, nw uint64

func (c *peerChannelScheduler) HandleData(b binmap.Bin, valid bool) error {
	// if (binmap.Bin(22).Contains(b) || b.Contains(22)) && c.s.debugHack {
	// 	log.Printf(">>> got data: bin %d valid %t", b, valid)
	// }
	// if (binmap.Bin(22).Contains(b) || b.Contains(22)) && !c.s.debugHack {
	// 	log.Printf(">>> some other channel got data: bin %d valid %t", b, valid)
	// }

	// tn := atomic.AddUint64(&nw, b.BaseLength())
	// c.s.lock.Lock()
	// for it := c.s.handledData.IterateFilledAt(b); it.Next(); {
	// 	n := atomic.AddUint64(&waste, it.Value().BaseLength())
	// 	cn := atomic.AddUint64(&c.waste, it.Value().BaseLength())
	// 	sn := atomic.AddUint64(&c.s.waste, it.Value().BaseLength())

	// 	// requested := c.requestSent.FilledAt(b)
	// 	// cancelled := c.cancelSent.FilledAt(b)
	// 	streams := make([]uint64, 0, it.Value().BaseLength())
	// 	for b, end := it.Value().Base(); b <= end; b += 2 {
	// 		sb := c.s.requestStreams[c.s.binStream(b)]
	// 		streams = append(streams, uint64(sb))
	// 	}

	// 	debug.RunEveryN(10, func() {
	// 		c.logger.Debug(
	// 			"waste",
	// 			zap.Uint16("stream", uint16(c.s.binStream(b))),
	// 			zap.Uint64("bin", uint64(b)),
	// 			zap.Uint64("received", tn),
	// 			zap.Uint64("waste", n),
	// 			zap.Uint64("peer_waste", sn),
	// 			zap.Uint64("channel_waste", cn),
	// 			// zap.Bool("requested", requested),
	// 			// zap.Bool("cancelled", cancelled),
	// 			zap.Uint64s("subbed", streams),
	// 		)
	// 	})
	// }
	// c.s.handledData.Set(b)
	// c.s.lock.Unlock()

	if !valid {
		// TODO: this should probably use a binmap so we can unset cancelled bins
		// TODO: this needs to account for chunks we receive from streams
		// c.lock.Lock()
		// if _, ok := c.requestTimes.Get(b); !ok {
		// 	return nil
		// }
		// c.lock.Unlock()

		c.s.lock.Lock()
		if !c.s.haveBins.FilledAt(b) {
			c.s.requestBins.Reset(b)
		}
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
		// LEDBAT rtt...
		rtt := float64((now.UnixNano() - ts) / timeGranularity)
		if c.dataRTTMean.Value() == 0 {
			c.dataRTTMean.Set(rtt)
			c.dataRTTVar.Set(rtt / 2)
		} else {
			c.dataRTTVar.Update(math.Abs(c.dataRTTMean.Value() - rtt))
			c.dataRTTMean.Update(rtt)
		}
		c.dataRTT.Update(rtt)

		c.etcp.OnAck(time.Duration(now.UnixNano() - ts))
		if l := b.BaseLength(); c.flightSize < l {
			c.flightSize = 0
		} else {
			c.flightSize -= l
		}
	}
	c.dataChunks.AddWithTime(b.BaseLength(), now)
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
		c.streamHaveLag[c.s.binStream(it.Value())].Update(float64((ts - first) / timeGranularity))
	}

	c.s.peerHaveBins.Set(b)
	c.peerHaveBins.Set(b)

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

		for b := c.s.peerMaxHaveBin; b <= br; b += 2 {
			if r, ok := c.s.requestStreams[c.s.binStream(b)]; ok && b >= r {
				c.s.requestBins.Set(b)
				// TODO: set timeout thing...
			}
		}

		c.s.peerMaxHaveBin = br
	}

	c.lock.Unlock()
	c.s.lock.Unlock()

	// debug.LogfEveryN(1000, "write1 count: %d time: %s avg: %d", nza, time.Duration(nzb), nzb/nza)

	return nil
}

func (c *peerChannelScheduler) HandleRequest(b binmap.Bin) error {
	// c.lock.Lock()
	// if !c.requested.EmptyAt(b) {
	// 	cancelled := !c.cancelled.EmptyAt(b)
	// 	log.Printf("waste - double requested cancelled %t", cancelled)
	// }
	// c.requested.Set(b)
	// c.lock.Unlock()

	c.p.pushData(c, b, peerPriorityLow)

	atomic.StoreUint32(&c.requestsAdded, 1)

	return nil
}

func (c *peerChannelScheduler) HandleCancel(b binmap.Bin) error {
	// c.lock.Lock()
	// c.cancelled.Set(b)
	// c.lock.Unlock()

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
	c.s.lock.Lock()
	c.lock.Lock()

	c.addStreamOpen(s, b)

	it := c.s.haveBins.IterateFilled()
	for ok := it.NextBaseAfter(b); ok; ok = it.NextBase() {
		if c.s.binStream(it.Value()) == s {
			c.p.pushData(c, it.Value(), peerPriorityHigh)
		}
	}

	c.lock.Unlock()
	c.s.lock.Unlock()
	return nil
}

func (c *peerChannelScheduler) HandleStreamCancel(s codec.Stream) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.addStreamClose(s)
	return nil
}

func (c *peerChannelScheduler) HandleStreamOpen(s codec.Stream, b binmap.Bin) error {
	c.s.lock.Lock()
	c.lock.Lock()
	// add to stream set

	/*
		when do we want to allow this?
			* it was requested
			* we have no information?
			* the peer is already the fastest known peer (ie seeds?)
			* this peer already sends us >threshold data?
			* we have no existing subscription for this stream?
	*/

	// if s == c.s.binStream(22) && c.s.debugHack {
	// 	log.Printf(">>> received stream open for s %d bin %d", s, b)
	// }

	c.requestStreams[s] = b
	c.s.requestStreams[s] = b

	// c.testSkip = true

	c.lock.Unlock()
	c.s.lock.Unlock()
	return nil
}

func (c *peerChannelScheduler) HandleStreamClose(s codec.Stream) error {
	c.s.lock.Lock()
	c.lock.Lock()

	// remove from stream set
	delete(c.requestStreams, s)
	delete(c.s.requestStreams, s)

	// reset outstanding requests
	// iterate through the bins in s > first empty in swarm bins

	c.lock.Unlock()
	c.s.lock.Unlock()
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
