// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"errors"
	"fmt"
	"log"
	"math"
	"math/bits"
	"math/rand"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ledbat"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/mathutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/MemeLabs/strims/pkg/stats"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var _ swarmScheduler = (*peerSwarmScheduler)(nil)
var _ channelScheduler = (*peerChannelScheduler)(nil)

const (
	schedulerGCInterval          = 5 * time.Second
	schedulerRateUpdateInterval  = 1 * time.Second
	schedulerStreamCheckInterval = 5 * time.Second
	schedulerRestartCooldown     = 1 * time.Second

	timeGranularity   = timeutil.Precision
	minRTTVar         = 200 * time.Millisecond
	minRequestTimeout = 100 * time.Millisecond
	maxRequestTimeout = 500 * time.Millisecond
)

// var debugHackCounter int32

func newPeerSchedulerStreamSubscription(n int) []peerSchedulerStream {
	s := make([]peerSchedulerStream, n)
	for i := range s {
		s[i].receivedChunkRate = stats.NewSMA(50, 200*time.Millisecond)
	}
	return s
}

type peerSchedulerStreamSubscription struct {
	startBin binmap.Bin
	channel  *peerChannelScheduler
}

type peerSchedulerStreamReceivedChunks struct {
	m         uint64
	next, max uint64
}

func (s *peerSchedulerStreamReceivedChunks) addReceivedChunk(o uint64) (unique, ok bool) {
	if o == s.next && o == s.max {
		s.next++
		s.max++
		return true, true
	}

	if o < s.next {
		// the bin is either not in the stream or has already arrived.
		return false, true
	}

	i := uint64(1) << (o - s.next)
	if i == 0 {
		// bins are arriving too far out of order to track
		return false, false
	}

	if s.m&i != 0 {
		// the bin was already received.
		return false, true
	}
	s.m |= i

	if o == s.next {
		n := uint64(bits.TrailingZeros64(^s.m))
		s.m >>= n
		s.next += n
	}
	if o >= s.max {
		s.max = o + 1
	}

	return true, true
}

type peerSchedulerStream struct {
	peerSchedulerStreamReceivedChunks
	source            *peerSchedulerStreamSubscription
	receivedChunkRate stats.SMA
	receivedChunkLag  stats.Welford
	peerHaveMax       binmap.Bin
	subscribers       []peerSchedulerStreamSubscription
}

func (s *peerSchedulerStream) setSource(cs *peerChannelScheduler, b binmap.Bin, o uint64, stream codec.Stream) {
	s.source = &peerSchedulerStreamSubscription{
		startBin: b,
		channel:  cs,
	}

	s.peerSchedulerStreamReceivedChunks.m = 0
	s.peerSchedulerStreamReceivedChunks.next = o
	s.peerSchedulerStreamReceivedChunks.max = o
}

func (s *peerSchedulerStream) addReceivedChunk(o uint64, t timeutil.Time, d time.Duration) {
	if s.source != nil {
		s.receivedChunkRate.AddWithTime(1, t)
		s.peerSchedulerStreamReceivedChunks.addReceivedChunk(o)
	}
	s.receivedChunkLag.Update(float64(d))
}

func (s *peerSchedulerStream) updatePeerHaveMax(b binmap.Bin) {
	if b > s.peerHaveMax {
		s.peerHaveMax = b
	}
}

func (s *peerSchedulerStream) resetSource() {
	s.source = nil
}

func (s *peerSchedulerStream) addSubscriber(cs *peerChannelScheduler, b binmap.Bin) {
	s.removeSubscriber(cs)
	s.subscribers = append(s.subscribers, peerSchedulerStreamSubscription{
		startBin: b,
		channel:  cs,
	})
}

func (s *peerSchedulerStream) removeSubscriber(cs *peerChannelScheduler) {
	for i := range s.subscribers {
		if s.subscribers[i].channel == cs {
			l := len(s.subscribers) - 1
			s.subscribers[i] = s.subscribers[l]
			s.subscribers[l] = peerSchedulerStreamSubscription{}
			s.subscribers = s.subscribers[:l]
			return
		}
	}
}

func newPeerSwarmScheduler(logger *zap.Logger, s *Swarm) *peerSwarmScheduler {
	// debugHack := atomic.AddInt32(&debugHackCounter, 1)
	// logger.Debug("started", zap.Int32("debugHack", debugHack))

	var signatureLayer int
	if s.options.Integrity.ProtectionMethod == integrity.ProtectionMethodMerkleTree {
		signatureLayer = bits.TrailingZeros16(uint16(s.options.ChunksPerSignature))
	}

	haveBins := s.store.Bins()

	ranks := make([]codec.Stream, s.options.StreamCount)
	for i := range ranks {
		ranks[i] = codec.Stream(i)
	}
	rand.Shuffle(len(ranks), func(i, j int) { ranks[i], ranks[j] = ranks[j], ranks[i] })

	return &peerSwarmScheduler{
		logger: logger,
		swarm:  s,

		streamCount:    codec.Stream(s.options.StreamCount),
		streamLayer:    uint64(bits.TrailingZeros16(uint16(s.options.StreamCount))),
		streamBits:     uint64(s.options.StreamCount - 1),
		signatureLayer: uint64(signatureLayer),

		peerHaveChunkRate: stats.NewSMA(15, time.Second),

		haveBins: haveBins,

		requestBins: haveBins.Clone(),
		streams:     newPeerSchedulerStreamSubscription(s.options.StreamCount),
		channels:    map[peerTaskQueue]*peerChannelScheduler{},

		integrityOverhead: s.options.IntegrityVerifierOptions().MaxMessageBytes(),
		chunkSize:         int(s.options.ChunkSize),
		liveWindow:        binmap.Bin(s.options.LiveWindow * 2),

		// debugHack: debugHack == 2,
		firstChunkSet: s.options.SchedulingMethod == SeedSchedulingMethod || !s.store.Empty(),

		// HAX
		nextGCTime:          timeutil.Now().Add(time.Duration(rand.Intn(5000)) * time.Millisecond),
		nextStreamCheckTime: timeutil.Now().Add(time.Duration(rand.Intn(3000)) * time.Millisecond),

		ranks: ranks,
	}
}

type peerSwarmScheduler struct {
	logger *zap.Logger
	swarm  *Swarm

	lock sync.Mutex

	streamCount    codec.Stream
	streamLayer    uint64
	streamBits     uint64
	signatureLayer uint64
	binTimes       timeSet

	peerHaveChunkRate stats.SMA
	peerMaxHaveBin    binmap.Bin

	haveBins     *binmap.Map
	haveBinMax   binmap.Bin
	wastedChunks uint64

	requestBins *binmap.Map
	streams     []peerSchedulerStream
	channels    map[peerTaskQueue]*peerChannelScheduler

	integrityOverhead int
	chunkSize         int
	liveWindow        binmap.Bin

	// debugHack     bool
	firstChunkSet bool

	nextGCTime          timeutil.Time
	nextStreamCheckTime timeutil.Time

	ranks []codec.Stream
}

func (s *peerSwarmScheduler) Run(t timeutil.Time) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if t.After(s.nextGCTime) {
		s.nextGCTime = t.Add(schedulerGCInterval)
		s.gc(t)
	}

	if t.After(s.nextStreamCheckTime) {
		s.nextStreamCheckTime = t.Add(schedulerStreamCheckInterval)
		s.checkStreams(t)
	}

	for _, cs := range s.channels {
		cs.timeOutRequests()
		cs.tryRestart(t)
	}

	// when the bitrate is low worry less about who we subscribe to

	// replace underperforming peers...
	// collect peers that swarms could do without?
	// handle this in runner based on ingress size?
	// unique but slow peers are also important..?

	if !s.firstChunkSet && !s.haveBins.Empty() {
		s.firstChunkSet = true

		var b binmap.Bin
		switch s.swarm.options.DeliveryMode {
		case BestEffortDeliveryMode:
			// TODO: this is a bit arbitrary... add backfill target swarm option?
			if halfWindow := s.liveWindow / 4 * 2; halfWindow < s.haveBinMax {
				b = s.haveBinMax - halfWindow
			}
		case LowLatencyDeliveryMode:
			b = s.haveBinMax
		}
		s.requestBins.FillBefore(b)
		s.swarm.store.SetOffset(b)

		s.logger.Debug(
			"set request offset",
			zap.Uint64("bin", uint64(b)),
			zap.Uint64("offset", b.BaseOffset()*uint64(s.chunkSize)),
		)

		for p, cs := range s.channels {
			p.Enqueue(cs)
		}
	}
}

func (s *peerSwarmScheduler) checkStreams(t timeutil.Time) {
	streamRate := s.peerHaveChunkRate.RateWithTime(time.Second, t) / uint64(s.streamCount)
	if streamRate == 0 {
		return
	}

	currentChannels := make([]*peerChannelScheduler, len(s.streams))
	currentLags := make([]stats.Welford, len(s.streams))
	for i, stream := range s.streams {
		currentLags[i] = s.streams[i].receivedChunkLag
		s.streams[i].receivedChunkLag.Reset()

		if stream.source != nil {
			currentChannels[i] = stream.source.channel
		}
	}

	var candidates []*peerChannelScheduler
	var candidateCaps []int64
	var candidateLags [][]stats.Welford
	var allLags [][]stats.Welford
	var allPeers []*peerChannelScheduler
	for _, cs := range s.channels {
		lag := make([]stats.Welford, len(s.streams))

		cs.lock.Lock()
		cap := int64(cs.dataRTT.SampleRateWithTime(time.Second, t) / streamRate)
		if cap > 1 {
			cap = 1
		}

		for j := 0; j < len(s.streams); j++ {
			assigned := cs == currentChannels[j]
			if assigned {
				cap++
			}
			if !cs.choked || assigned {
				lag[j] = cs.streamHaveLag[j]
			}
			cs.streamHaveLag[j].Reset()
		}

		allPeers = append(allPeers, cs)
		allLags = append(allLags, lag)
		if cap > 0 {
			candidates = append(candidates, cs)
			candidateCaps = append(candidateCaps, cap)
			candidateLags = append(candidateLags, lag)
		}
		cs.lock.Unlock()
	}
	if len(candidates) == 0 {
		return
	}

	s.logger.Debug(
		"stream candidates",
		zap.Int("viable", len(candidates)),
		zap.Int64s("capacities", candidateCaps),
	)

	temp := make([]stats.Welford, len(candidates))
	lag := make([]stats.Welford, len(s.streams))
	for i := 0; i < len(s.streams); i++ {
		for j := 0; j < len(candidates); j++ {
			temp[j] = candidateLags[j][i]
		}
		lag[i] = stats.WelfordMerge(temp...)
	}
	candidates = append(candidates, nil)
	candidateCaps = append(candidateCaps, int64(len(s.streams)))
	candidateLags = append(candidateLags, lag)

	// rank streams by aggregate lag in descending order
	sort.Slice(s.ranks, func(i, j int) bool {
		return lag[s.ranks[i]].Mean() > lag[s.ranks[j]].Mean()
	})

	return

	fmt.Printf(">>> ranks %d\n", s.ranks)

	var sb0, sb1, sb2, sb3 strings.Builder
	for i, lags := range allLags {
		if i > 0 {
			sb0.WriteByte('\n')
			sb1.WriteByte('\n')
			sb2.WriteByte('\n')
			sb3.WriteByte('\n')
		}
		sb0.WriteString(">>> ")
		sb1.WriteString(">>> ")
		sb2.WriteString(">>> ")
		sb3.WriteString(">>> ")

		fmt.Fprintf(&sb0, "%x", allPeers[i].p.ID())
		fmt.Fprintf(&sb1, "%x", allPeers[i].p.ID())
		fmt.Fprintf(&sb2, "%x", allPeers[i].p.ID())
		fmt.Fprintf(&sb3, "%x", allPeers[i].p.ID())

		var ahead float64
		for j, lag := range lags {
			fmt.Fprintf(&sb0, "%15.0f", lag.Mean())
			fmt.Fprintf(&sb1, "%15.0f", lag.StdDev())
			fmt.Fprintf(&sb2, "% 15d", int64(lag.Count()))
			fmt.Fprintf(&sb3, "% 15d", mathutil.Clamp(int(lag.Mean()-currentLags[j].Mean()), -1, 1))

			if lag.Mean() > currentLags[j].Mean() {
				ahead++
			}
		}
		fmt.Fprintf(&sb3, "% 15f", ahead/float64(s.streamCount)*100)
	}
	log.Println(">>> ---")
	fmt.Println(">>> mean")
	fmt.Println(sb0.String())
	fmt.Println(">>> stddev")
	fmt.Println(sb1.String())
	fmt.Println(">>> count")
	fmt.Println(sb2.String())
	fmt.Println(">>> order")
	fmt.Println(sb3.String())

	var lsb strings.Builder
	fmt.Fprintf(&lsb, "<<< %15s %15s %15s %15s %15s", "cto", "rtt", "rtt var", "cwnd", "flight size")
	for _, cs := range s.channels {
		lsb.WriteString("\n<<< ")
		cs.lock.Lock()
		fmt.Fprintf(&lsb, "%x", cs.p.ID())
		fmt.Fprintf(&lsb, "%15s", cs.ledbat.CTO())
		fmt.Fprintf(&lsb, "%15s", cs.ledbat.RTTMean())
		fmt.Fprintf(&lsb, "%15s", cs.ledbat.RTTVar())
		fmt.Fprintf(&lsb, "%15d", cs.ledbat.CWND())
		fmt.Fprintf(&lsb, "%15d", cs.ledbat.FlightSize())
		cs.lock.Unlock()
	}
	log.Printf("<<< --- %p", s)
	fmt.Println(lsb.String())

	assigner := newPeerStreamAssigner(int64(s.streamCount), candidateCaps)

	for i := 0; i < len(s.streams); i++ {
		current := currentLags[i]
		if current.Count() == 0 {
			continue
		}

		for j := 0; j < len(candidates); j++ {
			candidate := candidateLags[j][i]
			// if candidate.Count() > 0 {
			// 	assigner.addCandidate(i, j, int64(candidate.Mean()))
			// }
			if candidate.Count() > 0 {
				assigner.addCandidate(int64(i), int64(j), int64(candidate.Mean()))
				// p := stats.TDistribution(stats.WelchTTest(candidate, current), stats.WelchSatterthwaite(candidate, current))
				// if p > 0.05 {
				// 	continue
				// }

				// l := s.logger
				// if cs := candidates[j]; cs != nil {
				// 	l = candidates[j].logger
				// }
				// l.Debug(
				// 	"found stream candidate",
				// 	zap.Int("stream", i),
				// 	zap.Object("current", candidateStats{current}),
				// 	zap.Object("candidate", candidateStats{candidate}),
				// 	// zap.String("p", strconv.FormatFloat(p, 'g', 5, 64)),
				// )

				// assigner.addCandidate(i, j)
			}
		}
	}

	_, assignments := assigner.run()

	return

	b := s.requestBins.FindLastFilled() + 2
	for _, a := range assignments {
		cs := candidates[a.channel]
		if ccs := currentChannels[a.stream]; ccs != nil {
			if cs == ccs {
				continue
			}

			ccs.logger.Debug(
				"unsubscribed from stream",
				zap.Int64("stream", a.stream),
				zap.Bool("unassigned", cs == nil),
			)

			ccs.lock.Lock()
			ccs.addStreamCancel(codec.Stream(a.stream))
			ccs.p.Enqueue(ccs)
			ccs.lock.Unlock()
		}

		if cs == nil {
			continue
		}

		cs.logger.Debug(
			"subscribed to stream",
			zap.Int64("stream", a.stream),
			zap.Uint64("bin", uint64(b)),
		)

		cs.lock.Lock()
		cs.ledbat.HackTest(s.chunkSize)
		s.doStreamSub(cs, codec.Stream(a.stream), b)
		cs.p.Enqueue(cs)
		cs.lock.Unlock()
	}
}

type candidateStats struct {
	stats.Welford
}

// MarshalLogObject ...
func (s candidateStats) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddFloat64("count", s.Count())
	e.AddDuration("mean", time.Duration(s.Mean()).Round(time.Millisecond))
	e.AddDuration("stddev", time.Duration(s.StdDev()).Round(time.Millisecond))
	return nil
}

func (s *peerSwarmScheduler) doStreamSub(cs *peerChannelScheduler, stream codec.Stream, startBin binmap.Bin) {
	// s.setHasStream(stream)

	s.setStreamSource(cs, stream, startBin)
	cs.addStreamRequest(stream, startBin)

	// fill stream bins from last requested bin until last seen bin

	so := streamBinOffset(s.streamCount)
	min := startBin/so*so + streamBinOffset(stream)
	if min < startBin {
		min += so
	}
	for b := min; b <= s.peerMaxHaveBin; b += so {
		s.requestBins.Set(b)
	}
}

func (s *peerSwarmScheduler) setStreamSource(cs *peerChannelScheduler, stream codec.Stream, b binmap.Bin) {
	s.streams[stream].setSource(cs, b, s.binStreamOffset(b), stream)

	it := s.haveBins.IterateFilled()
	for ok := it.NextBaseAfter(b); ok; ok = it.NextBase() {
		if s.binStream(it.Value()) == stream {
			s.streams[stream].peerSchedulerStreamReceivedChunks.addReceivedChunk(s.binStreamOffset(it.Value()))
		}
	}
}

func (s *peerSwarmScheduler) setBinTime(b binmap.Bin, t timeutil.Time) {
	// once any bin in the signature is available we can assume that all of them
	// have been produced by the seed.
	//
	// depending on the seed bandwidth and stream count vs chunks per signature
	// this may increase the variance of peer have times but experimentally it
	// doesn't prevent subscriptions.
	//
	// if this works binTimes could be replaced with a ring buffer.
	if b.Layer() < s.signatureLayer {
		b = b.LayerShift(s.signatureLayer)
	}
	s.binTimes.Set(b, t)
}

func (s *peerSwarmScheduler) gc(t timeutil.Time) {
	// TODO: store this so we don't record times for out of bounds haves
	// TODO: base this on est 99th percentile peer have lag?
	if s.liveWindow < s.haveBinMax {
		s.binTimes.Prune(s.haveBinMax - s.liveWindow)
	}

	requestTimesThreshold := s.swarm.store.Next()
	for _, cs := range s.channels {
		cs.lock.Lock()
		cs.requestTimes.Prune(requestTimesThreshold)

		// if !cs.sentBinMin.IsNone() {
		// 	n0 := cs.sentBinTimes.Size()
		// 	cs.sentBinTimes.Prune(cs.sentBinMin)
		// 	n1 := cs.sentBinTimes.Size()
		// 	cs.logger.Debug(
		// 		"pruned sent bin times",
		// 		zap.Uint64("threshold", uint64(cs.sentBinMin)),
		// 		zap.Int("removed", n0-n1),
		// 		zap.Int("remaining", n1),
		// 		zap.Duration("rtt", time.Duration(cs.sendRTT.Value()).Round(time.Millisecond)),
		// 	)
		// 	cs.sentBinMin = binmap.None
		// }
		cs.lock.Unlock()
	}
}

func (s *peerSwarmScheduler) Reset() {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.peerHaveChunkRate = stats.NewSMA(15, time.Second)
	s.haveBins = binmap.New()
	s.requestBins = binmap.New()
	s.streams = newPeerSchedulerStreamSubscription(int(s.streamCount))
	s.firstChunkSet = false

	for _, cs := range s.channels {
		cs.lock.Lock()

		cs.streamHaveLag = make([]stats.Welford, s.streamCount)
		cs.dataChunks = stats.NewSMA(15, time.Second)
		cs.haveBins = binmap.New()
		cs.cancelBins = binmap.New()
		cs.requestStreams = make([]binmap.Bin, s.streamCount)
		cs.extraMessages = []codec.Message{newHandshake(s.swarm)}
		cs.peerHaveBins = binmap.New()
		cs.nextRestartTime = timeutil.Now().Add(schedulerRestartCooldown)
		cs.ledbat = ledbat.New()
		cs.handshakeReceived = false

		cs.p.EnqueueNow(cs)

		cs.lock.Unlock()
	}
}

func (s *peerSwarmScheduler) Consume(c store.Chunk) {
	s.lock.Lock()
	defer s.lock.Unlock()

	t := timeutil.Now()
	s.setBinTime(c.Bin, t)

	wastedChunks := c.Bin.BaseLength()

	for it := s.haveBins.IterateEmptyAt(c.Bin); it.NextBase(); {
		_, first, _ := s.binTimes.Get(it.Value())
		stream := &s.streams[s.binStream(it.Value())]
		stream.addReceivedChunk(s.binStreamOffset(it.Value()), t, t.Sub(first))
		stream.updatePeerHaveMax(it.Value())

		wastedChunks--

		for _, sub := range stream.subscribers {
			if sub.startBin <= it.Value() {
				sub.channel.p.PushData(sub.channel, it.Value(), timeutil.EpochTime, peerPriorityHigh)
			}
		}
	}

	s.wastedChunks += wastedChunks

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

	s.requestBins.Set(c.Bin)
}

func (s *peerSwarmScheduler) ChannelScheduler(p peerTaskQueue, cw codecMessageWriter) channelScheduler {
	s.lock.Lock()
	defer s.lock.Unlock()

	c := &peerChannelScheduler{
		logger:          s.logger.With(logutil.ByteHex("peer", p.ID())),
		p:               p,
		cw:              cw,
		s:               s,
		streamHaveLag:   make([]stats.Welford, s.streamCount),
		dataRTT:         stats.NewSMA(int(schedulerStreamCheckInterval/(100*time.Millisecond)), 100*time.Millisecond),
		dataRTTMean:     stats.NewEMA(0.125),
		dataRTTVar:      stats.NewEMA(0.25),
		dataChunks:      stats.NewSMA(15, time.Second),
		haveBins:        s.haveBins.Clone(),
		cancelBins:      binmap.New(),
		requestStreams:  make([]binmap.Bin, s.streamCount),
		extraMessages:   []codec.Message{newHandshake(s.swarm)},
		peerHaveBins:    binmap.New(),
		nextRestartTime: timeutil.Now().Add(schedulerRestartCooldown),
		// sentBinMin:     binmap.None,
		// sendRTT:        stats.NewSMA(50, 100*time.Millisecond),

		// test: qos.NewHLB(math.MaxFloat64),

		// etcp:   etcp.NewControl(),
		ledbat: ledbat.New(),

		// written:     binmap.New(),
		// cancelled:   binmap.New(),
		// requested:   binmap.New(),
		// requestSent: binmap.New(),
		// cancelSent:  binmap.New(),
		// pushedData:  binmap.New(),

		candidateBins: make([][]binmap.Bin, s.streamCount),
	}

	for i := codec.Stream(0); i < s.streamCount; i++ {
		c.requestStreams[i] = binmap.None
	}

	s.channels[p] = c

	p.Enqueue(c)

	return c
}

func (s *peerSwarmScheduler) CloseChannel(p peerTaskQueue) {
	s.lock.Lock()
	cs, ok := s.channels[p]
	if !ok {
		s.lock.Unlock()
		return
	}

	cs.lock.Lock()
	for i := range s.streams {
		stream := &s.streams[i]
		stream.removeSubscriber(cs)
		if stream.source != nil && stream.source.channel == cs {
			cs.closeStream(codec.Stream(i))
		}
	}

	cs.clearRequests()

	delete(s.channels, p)

	cs.lock.Unlock()
	s.lock.Unlock()

	p.RemoveRunner(cs)
}

func (s *peerSwarmScheduler) binStreamOffset(b binmap.Bin) uint64 {
	return uint64(b) >> (s.streamLayer + 1)
}

func (s *peerSwarmScheduler) binStream(b binmap.Bin) codec.Stream {
	return codec.Stream(uint64(b>>1) & s.streamBits)
}

// func (s *peerSwarmScheduler) binHasStream(b binmap.Bin) bool {
// 	return s.streamFills[b&s.streamBits] > 0
// }

// func (s *peerSwarmScheduler) setHasStream(v codec.Stream) {
// 	b := streamBinOffset(v)
// 	if s.streamFills[b] > 0 {
// 		return
// 	}

// 	for i := 0; i < s.streamLayer; i++ {
// 		s.streamFills[b]++
// 		b = b.Parent()
// 	}
// }

// func (s *peerSwarmScheduler) unsetHasStream(v codec.Stream) {
// 	b := streamBinOffset(v)
// 	if s.streamFills[b] == 0 {
// 		return
// 	}

// 	for i := 0; i < s.streamLayer; i++ {
// 		s.streamFills[b]--
// 		b = b.Parent()
// 	}
// }

type peerChannelScheduler struct {
	logger *zap.Logger
	p      peerTaskQueue
	cw     codecMessageWriter
	s      *peerSwarmScheduler

	peerTaskRunnerQueueTicket

	lock sync.Mutex

	handshakeSent bool
	choked        bool
	streamHaveLag []stats.Welford
	requestTimes  timeSet
	dataRTT       stats.SMA
	dataRTTMean   stats.EMA
	dataRTTVar    stats.EMA
	dataChunks    stats.SMA

	haveBins       *binmap.Map // bins to send HAVEs for
	cancelBins     *binmap.Map // bins to send CANCELs for
	requestBins    binQueue    // bins recently requested from the peer
	requestStreams []binmap.Bin
	extraMessages  []codec.Message

	enqueueNow     uint32
	peerLiveWindow binmap.Bin
	peerMaxHaveBin binmap.Bin
	peerHaveBins   *binmap.Map // bins the peer claims to have

	// sentBinTimes timeSet
	// sentBinMin   binmap.Bin
	// sendRTT      stats.SMA

	// test *qos.HLB
	// testSkip bool

	// etcp       *etcp.Control
	ledbat *ledbat.Controller
	// flightSize uint64

	nextRestartTime   timeutil.Time
	handshakeReceived bool

	nextPingTime timeutil.Time
	pingNonce    uint64

	waste uint64
	// written     *binmap.Map
	// cancelled   *binmap.Map
	// requested   *binmap.Map
	// requestSent *binmap.Map
	// cancelSent  *binmap.Map
	// pushedData  *binmap.Map

	candidateBins [][]binmap.Bin
}

func (c *peerChannelScheduler) tryRestart(t timeutil.Time) {
	c.lock.Lock()
	defer c.lock.Unlock()

	if !c.handshakeReceived && t.After(c.nextRestartTime) {
		c.nextRestartTime = t.Add(schedulerRestartCooldown)
		c.extraMessages = append(c.extraMessages, &codec.Restart{})
		c.p.Enqueue(c)
	}
}

func (c *peerChannelScheduler) appendHaveBins(hb binmap.Bin) {
	c.lock.Lock()
	c.haveBins.Set(hb)
	c.lock.Unlock()

	c.p.Enqueue(c)
}

func (c *peerChannelScheduler) timeOutRequests() {
	c.lock.Lock()
	defer c.lock.Unlock()

	var n uint64
	var bins []uint64

	for ri := c.requestBins.IterateLessThan(timeutil.Now()); ri.Next(); {
		for ei := c.s.haveBins.IterateEmptyAt(ri.Value()); ei.NextBase(); {
			// if (binmap.Bin(22).Contains(ei.Value()) || ei.Value().Contains(22)) && c.s.debugHack {
			// 	log.Println(">>> timed out request")
			// }

			c.s.requestBins.Reset(ei.Value())
			c.requestTimes.Unset(ei.Value())
			c.cancelBins.Set(ei.Value())
			n++
			bins = append(bins, uint64(ei.Value()))
		}
	}

	if n > 0 {
		c.logger.Debug(
			"timed out requests",
			zap.Uint64("chunks", n),
			// zap.Uint64s("bins", bins),
		)

		// c.etcp.OnDataLoss()
		c.ledbat.AddDataLoss(int(n)*c.s.chunkSize, false)
		// if c.flightSize < n {
		// 	c.flightSize = 0
		// } else {
		// 	c.flightSize -= n
		// }

		c.p.Enqueue(c)
	}
}

func (c *peerChannelScheduler) clearRequests() {
	for ri := c.requestBins.IterateLessThan(timeutil.MaxTime); ri.Next(); {
		for ei := c.s.haveBins.IterateEmptyAt(ri.Value()); ei.NextBase(); {
			c.s.requestBins.Reset(ei.Value())
		}
	}
}

func (c *peerChannelScheduler) WriteData(b binmap.Bin, t timeutil.Time, pri peerPriority) (int, error) {
	for n := c.cw.Available(); n < int(b.BaseLength())*c.s.chunkSize+c.s.integrityOverhead; {
		if b.IsBase() {
			c.p.PushFrontData(c, b, t, pri)
			return 0, codec.ErrNotEnoughSpace
		}

		c.p.PushFrontData(c, b.Right(), t, pri)
		b = b.Left()
	}

	c.lock.Lock()
	_, err := c.s.swarm.verifier.WriteIntegrity(b, c.peerHaveBins, c.cw)
	c.lock.Unlock()
	if err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.PushFrontData(c, b, t, pri)
		} else {
			c.logger.Debug(
				"error writing integrity",
				zap.Uint64("bin", uint64(b)),
				zap.Stringer("priority", pri),
				zap.Uint16("stream", uint16(c.s.binStream(b))),
				zap.Error(err),
			)
		}
		return 0, err
	}
	if _, err := c.s.swarm.store.WriteData(b, timeutil.Now(), c.cw); err != nil {
		if errors.Is(err, codec.ErrNotEnoughSpace) {
			c.cw.Reset()
			c.p.PushFrontData(c, b, t, pri)
		} else {
			c.logger.Debug(
				"error writing data",
				zap.Uint64("bin", uint64(b)),
				zap.Stringer("priority", pri),
				zap.Uint16("stream", uint16(c.s.binStream(b))),
				zap.Error(err),
			)
		}
		return 0, err
	}

	// now := timeutil.Now()
	// c.lock.Lock()
	// c.sentBinTimes.Set(b, now)

	// if bl := b.BaseLeft(); c.sentBinMin > bl {
	// 	c.sentBinMin = bl
	// }
	// c.lock.Unlock()

	return c.flushWrites()
}

func (c *peerChannelScheduler) Write() (int, error) {
	if now := timeutil.Now(); now.After(c.nextPingTime) {
		_, err := c.cw.WritePing(codec.Ping{
			Nonce: codec.Nonce{
				Value: uint64(now.UnixNano()),
			},
		})
		if err != nil {
			return 0, err
		}
		c.nextPingTime = now.Add(time.Second)
		c.pingNonce = uint64(now.UnixNano())
	}

	err := c.write0()
	if err == nil {
		err = c.write1()
	}
	if err != nil && !errors.Is(err, codec.ErrNotEnoughSpace) {
		return 0, err
	}

	n, ferr := c.flushWrites()
	if ferr != nil {
		return n, ferr
	}
	return n, err
}

func (c *peerChannelScheduler) write0() error {
	c.lock.Lock()
	defer c.lock.Unlock()

	for i, m := range c.extraMessages {
		var err error
		switch m := m.(type) {
		case *codec.Handshake:
			_, err = c.cw.WriteHandshake(*m)
		case *codec.Restart:
			_, err = c.cw.WriteRestart(*m)
		case *codec.StreamRequest:
			_, err = c.cw.WriteStreamRequest(*m)
		case *codec.StreamCancel:
			_, err = c.cw.WriteStreamCancel(*m)
		case *codec.StreamOpen:
			_, err = c.cw.WriteStreamOpen(*m)
		case *codec.StreamClose:
			_, err = c.cw.WriteStreamClose(*m)
		case *codec.Pong:
			_, err = c.cw.WritePong(*m)
		}

		if err != nil {
			c.pruneExtraMessages(i)
			return err
		}
	}
	c.pruneExtraMessages(len(c.extraMessages))

	if err := c.writeMapBins(c.haveBins, c.writeHave); err != nil {
		return err
	}

	if err := c.writeMapBins(c.cancelBins, c.writeCancel); err != nil {
		return err
	}

	return nil
}

func (c *peerChannelScheduler) write1() error {
	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if c.choked {
		return nil
	}

	var min binmap.Bin
	if c.peerMaxHaveBin > c.peerLiveWindow {
		min = c.peerMaxHaveBin - c.peerLiveWindow
	}
	// if m := c.s.haveBinMax - c.s.liveWindow; min > m {
	// 	min = m
	// }
	if m := c.s.swarm.store.Next(); min > m {
		min = m
	}
	// TODO: skip reqs until firstChunkSet?
	if !c.s.firstChunkSet && c.peerMaxHaveBin > min {
		min = c.peerMaxHaveBin
	}

	now := timeutil.Now()
	timeout := now.Add(c.ledbat.CTO())
	// timeout := now.Add(c.requestTimeout())
	// debug.LogfEveryN(
	// 	100,
	// 	"request timeout %s flightSize: %d cwnd %d",
	// 	c.requestTimeout(),
	// 	c.flightSize,
	// 	c.etcp.CWND(),
	// )

	for i := range c.candidateBins {
		c.candidateBins[i] = c.candidateBins[i][:0]
	}

	var err error

	// n := uint64(c.etcp.CWND()) - c.flightSize
	n := uint64(mathutil.Max(c.ledbat.CWND()-c.ledbat.FlightSize(), 0) / c.s.chunkSize)
	if n > 0 {
		it := binmap.NewIntersectionIterator(
			c.s.requestBins.IterateEmptyAt(c.peerHaveBins.RootBin()),
			c.peerHaveBins.IterateFilled(),
		)

		for ok := it.NextAfter(min); ok; ok = it.Next() {
			for b, br := it.Value().Base(); b <= br; b += 2 {
				rank := c.s.ranks[c.s.binStream(b)]
				c.candidateBins[rank] = append(c.candidateBins[rank], b)
			}
		}

	EachRank:
		for _, bins := range c.candidateBins {
			for _, b := range bins {
				_, err = c.cw.WriteRequest(codec.Request{
					Address:   codec.Address(b),
					Timestamp: codec.Timestamp{Time: now},
				})
				if err != nil {
					break EachRank
				}

				// c.requestSent.Set(b)
				c.s.requestBins.Set(b)
				c.requestTimes.Set(b, now)
				c.requestBins.Push(b, timeout)
				c.ledbat.AddSent(int(b.BaseLength()) * c.s.chunkSize)

				n--
				if n == 0 {
					break EachRank
				}
			}
		}
		// EachUnrequestedBin:
		// 	for ok := it.NextAfter(min); ok; ok = it.Next() {
		// 		b := it.Value()
		// 		br := b.BaseRight()
		// 		for ; b <= br; b = b.LayerRight() {
		// 			if nl := uint64(bits.Len64(n)) - 1; b.Layer() > nl {
		// 				b = b.LayerShift(nl)
		// 			}

		// 			_, err = c.cw.WriteRequest(codec.Request{
		// 				Address:   codec.Address(b),
		// 				Timestamp: codec.Timestamp{Time: now},
		// 			})
		// 			if err != nil {
		// 				break EachUnrequestedBin
		// 			}

		// 			// c.requestSent.Set(b)
		// 			c.s.requestBins.Set(b)
		// 			c.requestTimes.Set(b, now)
		// 			c.requestBins.Push(b, timeout)
		// 			// c.flightSize += b.BaseLength()
		// 			c.ledbat.AddSent(int(b.BaseLength()) * c.s.chunkSize)

		// 			n -= b.BaseLength()
		// 			if n == 0 {
		// 				break EachUnrequestedBin
		// 			}
		// 		}
		// 	}
	}

	return err
}

func (c *peerChannelScheduler) requestTimeout() time.Duration {
	rtt := c.dataRTTMean.Value()
	if rtt == 0 {
		return maxRequestTimeout
	}
	return time.Duration(rtt + math.Max(float64(minRTTVar), 4*c.dataRTTVar.Value()))

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
	c.requestStreams[s] = b

	c.extraMessages = append(c.extraMessages, &codec.StreamRequest{
		StreamAddress: codec.StreamAddress{
			Stream:  s,
			Address: codec.Address(b),
		},
	})
}

func (c *peerChannelScheduler) addStreamCancel(s codec.Stream) {
	c.closeStream(s)

	c.extraMessages = append(c.extraMessages, &codec.StreamCancel{
		Stream: s,
	})
}

func (c *peerChannelScheduler) addStreamOpen(s codec.Stream, b binmap.Bin) {
	if t := c.s.swarm.store.Tail(); b < t {
		b = t
	}

	c.s.streams[s].addSubscriber(c, b)

	it := c.s.haveBins.IterateFilled()
	for ok := it.NextBaseAfter(b); ok; ok = it.NextBase() {
		if c.s.binStream(it.Value()) == s {
			c.p.PushData(c, it.Value(), timeutil.EpochTime, peerPriorityHigh)
		}
	}

	c.extraMessages = append(c.extraMessages, &codec.StreamOpen{
		StreamAddress: codec.StreamAddress{
			Stream:  s,
			Address: codec.Address(b),
		},
	})
}

func (c *peerChannelScheduler) closeStream(s codec.Stream) {
	// remove from stream set
	if s < c.s.streamCount && !c.requestStreams[s].IsNone() {
		it := binmap.NewIntersectionIterator(
			c.s.haveBins.IterateEmptyAt(c.s.requestBins.RootBin()),
			c.s.requestBins.IterateFilled(),
		)
		for it.NextBase() {
			if c.requestStreams[s] <= it.Value() && c.s.binStream(it.Value()) == s {
				c.s.requestBins.Reset(it.Value())
			}
		}

		c.requestStreams[s] = binmap.None
		c.s.streams[s].resetSource()
	}
}

func (c *peerChannelScheduler) addStreamClose(s codec.Stream) {
	// delete enqueued sends in this stream
	c.s.streams[s].removeSubscriber(c)

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

func (c *peerChannelScheduler) ExpectData(b binmap.Bin) bool {
	// TODO: check that the data was either requested or part of a stream we
	// expect from this peer?

	return c.handshakeReceived
}

func (c *peerChannelScheduler) HandleHandshake(liveWindow uint32) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.peerLiveWindow = binmap.Bin(liveWindow * 2)
	c.handshakeReceived = true
	return nil
}

func (c *peerChannelScheduler) HandleRestart() error {
	c.lock.Lock()
	defer c.lock.Unlock()
	c.extraMessages = append(c.extraMessages, newHandshake(c.s.swarm))
	c.haveBins = c.s.haveBins.Clone()
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleAck(b binmap.Bin, delaySample time.Duration) error {
	// c.lock.Lock()
	// defer c.lock.Unlock()
	// ignore these for now?
	return nil
}

var tb, tc, td uint64
var waste, nw uint64

func (c *peerChannelScheduler) HandleData(b binmap.Bin, t timeutil.Time, valid bool) error {
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
	// 		var sb uint64
	// 		if s := c.s.requestStreams[c.s.binStream(b)]; s != nil {
	// 			sb = uint64(s.startBin)
	// 		}
	// 		streams = append(streams, sb)
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

	// ntc := atomic.AddUint64(&tc, 1)
	// ntb := atomic.AddUint64(&tb, b.BaseLength()*uint64(c.s.chunkSize))
	// if ntc%1000 == 0 {
	// 	log.Printf("HandleData bytes: %d chunks: %d", ntb, ntc)
	// }

	now := timeutil.Now()

	atomic.StoreUint32(&c.enqueueNow, 1)

	c.lock.Lock()
	defer c.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

	if _, ts, ok := c.requestTimes.Get(b); ok {
		// LEDBAT rtt...
		rtt := float64(now.Sub(ts))
		if c.dataRTTMean.Value() == 0 {
			c.dataRTTMean.Set(rtt)
			c.dataRTTVar.Set(rtt / 2)
		} else {
			c.dataRTTVar.Update(math.Abs(c.dataRTTMean.Value() - rtt))
			c.dataRTTMean.Update(rtt)
		}
		c.dataRTT.AddNWithTime(b.BaseLength(), uint64(rtt), now)

		// c.etcp.OnAck(now.Sub(ts))

		c.ledbat.AddDelaySample(now.Sub(t), int(b.BaseLength())*c.s.chunkSize)

		// if l := b.BaseLength(); c.flightSize < l {
		// 	c.flightSize = 0
		// } else {
		// 	c.flightSize -= l
		// }
	}
	c.dataChunks.AddWithTime(b.BaseLength(), now)
	return nil
}

var za, zb int64

func (c *peerChannelScheduler) HandleHave(b binmap.Bin) error {
	// TODO: reject far future/past bins

	t := timeutil.Now()

	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

	c.s.setBinTime(b, t)

	for it := c.peerHaveBins.IterateEmptyAt(b); it.NextBase(); {
		_, first, _ := c.s.binTimes.Get(it.Value())
		stream := c.s.binStream(it.Value())

		c.streamHaveLag[stream].Update(float64(t.Sub(first)))
		c.s.streams[stream].updatePeerHaveMax(it.Value())

		// sb, st, ok := c.sentBinTimes.Get(it.Value())
		// if ok {
		// 	c.sendRTT.AddNWithTime(sb.BaseLength(), uint64(t-st), t)
		// 	c.sentBinTimes.Unset(sb)
		// }
	}

	// c.s.peerHaveBins.Set(b)
	c.peerHaveBins.Set(b)

	br := b.BaseRight()
	if br > c.peerMaxHaveBin {
		c.peerMaxHaveBin = br
	}
	if br > c.s.peerMaxHaveBin {
		if c.s.peerMaxHaveBin != 0 {
			c.s.peerHaveChunkRate.AddWithTime(br.BaseOffset()-c.s.peerMaxHaveBin.BaseOffset(), t)
		}

		for b := c.s.peerMaxHaveBin; b <= br; b += 2 {
			if s := c.s.streams[c.s.binStream(b)].source; s != nil && b >= s.startBin {
				c.s.requestBins.Set(b)
				// TODO: set timeout thing...
			}
		}

		c.s.peerMaxHaveBin = br
	}

	// debug.LogfEveryN(1000, "write1 count: %d time: %s avg: %d", nza, time.Duration(nzb), nzb/nza)

	return nil
}

func (c *peerChannelScheduler) HandleRequest(b binmap.Bin, t timeutil.Time) error {
	// c.lock.Lock()
	// if !c.requested.EmptyAt(b) {
	// 	cancelled := !c.cancelled.EmptyAt(b)
	// 	log.Printf("waste - double requested cancelled %t", cancelled)
	// }
	// c.requested.Set(b)
	// c.lock.Unlock()

	c.lock.Lock()
	ok := c.handshakeReceived
	c.lock.Unlock()
	if !ok {
		return nil
	}

	c.p.PushData(c, b, t, peerPriorityLow)

	atomic.StoreUint32(&c.enqueueNow, 1)

	return nil
}

func (c *peerChannelScheduler) HandleCancel(b binmap.Bin) error {
	// c.lock.Lock()
	// c.cancelled.Set(b)
	// c.lock.Unlock()

	c.p.RemoveData(c, b, peerPriorityLow)

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
	c.extraMessages = append(c.extraMessages, &codec.Pong{
		Nonce: codec.Nonce{
			Value: nonce,
		},
	})
	atomic.StoreUint32(&c.enqueueNow, 1)
	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandlePong(nonce uint64) error {
	c.lock.Lock()
	defer c.lock.Unlock()
	// update rtt
	if nonce == c.pingNonce {
		c.ledbat.AddRTTSample(timeutil.Since(timeutil.Time(nonce)))
	}
	return nil
}

func (c *peerChannelScheduler) HandleStreamRequest(s codec.Stream, b binmap.Bin) error {
	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

	c.addStreamOpen(s, b)

	return nil
}

func (c *peerChannelScheduler) HandleStreamCancel(s codec.Stream) error {
	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

	// c.addStreamClose(s)
	c.s.streams[s].removeSubscriber(c)

	return nil
}

func (c *peerChannelScheduler) HandleStreamOpen(s codec.Stream, b binmap.Bin) error {
	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

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

	c.s.setStreamSource(c, s, b)

	return nil
}

func (c *peerChannelScheduler) HandleStreamClose(s codec.Stream) error {
	c.s.lock.Lock()
	c.lock.Lock()
	defer c.lock.Unlock()
	defer c.s.lock.Unlock()

	if !c.handshakeReceived {
		return nil
	}

	c.closeStream(s)

	return nil
}

// deprecated?
func (c *peerChannelScheduler) HandleMessageEnd() error {
	if atomic.CompareAndSwapUint32(&c.enqueueNow, 1, 0) {
		c.p.EnqueueNow(c)
	} else {
		c.p.Enqueue(c)
	}
	c.ledbat.DigestDelaySamples()

	return nil
}
