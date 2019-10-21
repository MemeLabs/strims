package encoding

import (
	"math/rand"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ema"
	"github.com/MemeLabs/go-ppspp/pkg/ledbat"
)

// TODO: this shouldn't be part of the public interface
// TODO: locking madness...

const minPingInterval = time.Second

// Peer ...
type Peer struct {
	sync.Mutex

	id                  int64
	conn                TransportConn
	channels            sync.Map
	lastActive          int64
	ledbat              *ledbat.Controller
	chunkIntervalMean   ema.Mean
	lastChunkTime       time.Time
	requestedChunkCount uint64
	sentChunkCount      uint64
	ackedChunkCount     uint64
	prevAckedChunkCount uint64
	receivedChunkCount  uint64
	cancelledChunkCount uint64
	rttSampleChannel    uint32
	rttSampleBin        binmap.Bin
	rttSampleTime       time.Time
}

// NewPeer ...
func NewPeer(id int64, conn TransportConn) *Peer {
	return &Peer{
		id:                id,
		conn:              conn,
		lastActive:        time.Now().Unix(),
		ledbat:            ledbat.New(),
		chunkIntervalMean: ema.New(0.05),
		rttSampleBin:      binmap.None,
	}
}

// UpdateLastActive ...
func (p *Peer) UpdateLastActive() {
	atomic.StoreInt64(&p.lastActive, time.Now().UnixNano())
}

// LastActive ...
func (p *Peer) LastActive() time.Time {
	return time.Unix(0, atomic.LoadInt64(&p.lastActive))
}

// OutstandingChunks ...
func (p *Peer) OutstandingChunks() int {
	return int(p.requestedChunkCount - (p.receivedChunkCount + p.cancelledChunkCount))
}

// AddRequestedChunks ...
func (p *Peer) AddRequestedChunks(n uint64) {
	p.requestedChunkCount += n
}

// AddAckedChunk ...
func (p *Peer) AddAckedChunk() {
	p.ackedChunkCount++
}

// NewlyAckedCount ...
func (p *Peer) NewlyAckedCount() uint64 {
	c := p.prevAckedChunkCount
	p.prevAckedChunkCount = p.ackedChunkCount
	return p.ackedChunkCount - c
}

// AddSentChunk ...
func (p *Peer) AddSentChunk() {
	p.ledbat.StartDebugging()
	p.sentChunkCount++
}

// AddCancelledChunk ...
func (p *Peer) AddCancelledChunk() {
	p.cancelledChunkCount++
}

// AddReceivedChunk ...
func (p *Peer) AddReceivedChunk() {
	p.receivedChunkCount++

	now := p.LastActive()
	ivl := now.Sub(p.lastChunkTime)
	if ivl != 0 && !p.lastChunkTime.IsZero() {
		p.chunkIntervalMean.Update(float64(ivl))
	}
	p.lastChunkTime = now
}

// TrackBinRTT ...
func (p *Peer) TrackBinRTT(cid uint32, b binmap.Bin) (ok bool) {
	if ok = p.rttSampleBin.IsNone(); ok {
		p.rttSampleChannel = cid
		p.rttSampleBin = b
		p.rttSampleTime = time.Now()
	}
	return
}

// TrackPingRTT ...
func (p *Peer) TrackPingRTT() (nonce uint64, ok bool) {
	if ok = time.Since(p.rttSampleTime) > minPingInterval; ok {
		// with even nonces Contains(nonce) is an equality check
		nonce = uint64(rand.Int63()) << 1

		p.rttSampleChannel = 0
		p.rttSampleBin = binmap.Bin(nonce)
		p.rttSampleTime = time.Now()
	}
	return
}

// AddRTTSample ...
func (p *Peer) AddRTTSample(cid uint32, b binmap.Bin) {
	if p.rttSampleChannel == cid && p.rttSampleBin.Contains(b) {
		p.ledbat.AddRTTSample(time.Since(p.rttSampleTime))
		p.rttSampleBin = binmap.None
	}
}

// ChunkIntervalMean ...
func (p *Peer) ChunkIntervalMean() time.Duration {
	return time.Duration(p.chunkIntervalMean.Value())
}

// Close ...
func (p *Peer) Close() {
	// TODO: send empty handshake (ppspp goodbye)
}
