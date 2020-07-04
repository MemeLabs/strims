package ppspp

import (
	"io"
	"math/rand"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ledbat"
	"github.com/MemeLabs/go-ppspp/pkg/ma"
)

// TODO: this shouldn't be part of the public interface
// TODO: locking madness...

// WriteFlushCloser ...
type WriteFlushCloser interface {
	io.WriteCloser
	MTU() int
	Flush() error
}

const minPingInterval = time.Second * 10

// Peer ...
type Peer struct {
	sync.Mutex

	lastActive          int64
	ledbat              *ledbat.Controller
	chunkIntervalMean   ma.Simple
	lastChunkTime       time.Time
	requestedChunkCount uint64
	receivedChunkCount  uint64
	cancelledChunkCount uint64
	rttSampleChannel    uint64
	rttSampleBin        binmap.Bin
	rttSampleTime       time.Time
	channels            map[*Swarm]*channelWriter
}

// NewPeer ...
func NewPeer() *Peer {
	return &Peer{
		lastActive:        iotime.Load().Unix(),
		ledbat:            ledbat.New(),
		chunkIntervalMean: ma.NewSimple(500, 10*time.Millisecond),
		rttSampleBin:      binmap.None,
		channels:          map[*Swarm]*channelWriter{},
	}
}

func (p *Peer) addChannel(s *Swarm, c *channelWriter) {
	p.Lock()
	p.channels[s] = c
	p.Unlock()
}

func (p *Peer) removeChannel(s *Swarm) {
	p.Lock()
	delete(p.channels, s)
	p.Unlock()
}

func (p *Peer) addDelaySample(sample time.Duration, chunkSize int) {
	p.Lock()
	p.ledbat.AddDelaySample(sample, chunkSize)
	p.Unlock()
}

func (p *Peer) addDataLoss(size int) {
	p.Lock()
	p.ledbat.AddDataLoss(size, false)
	p.Unlock()
}

// UpdateLastActive ...
func (p *Peer) UpdateLastActive() {
	atomic.StoreInt64(&p.lastActive, iotime.Load().UnixNano())
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

// AddCancelledChunk ...
func (p *Peer) AddCancelledChunk() {
	p.cancelledChunkCount++
}

func (p *Peer) addReceivedChunk() {
	p.Lock()
	p.receivedChunkCount++
	p.chunkIntervalMean.AddWithTime(1, iotime.Load())
	p.Unlock()
}

// TrackPingRTT ...
func (p *Peer) TrackPingRTT(cid uint64, t time.Time) (nonce uint64, ok bool) {
	if ok = t.Sub(p.rttSampleTime) > minPingInterval; ok {
		// with even nonces Contains(nonce) is an equality check
		nonce = uint64(rand.Int63()) << 1

		p.rttSampleChannel = cid
		p.rttSampleBin = binmap.Bin(nonce)
		p.rttSampleTime = t
	}
	return
}

func (p *Peer) addRTTSample(cid uint64, b binmap.Bin) {
	p.Lock()
	if p.rttSampleChannel == cid && p.rttSampleBin.Contains(b) {
		p.ledbat.AddRTTSample(iotime.Load().Sub(p.rttSampleTime))
		p.rttSampleBin = binmap.None
	}
	p.Unlock()
}

// ChunkIntervalMean ...
func (p *Peer) ChunkIntervalMean() time.Duration {
	return p.chunkIntervalMean.Interval()
}

// Close ...
func (p *Peer) Close() {
	// TODO: send empty handshake (ppspp goodbye)

	for s, c := range p.channels {
		s.removeChannel(p)
		c.Close()
	}
}
