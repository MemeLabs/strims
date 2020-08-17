package ppspp

import (
	"io"
	"math/rand"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ledbat"
	"github.com/MemeLabs/go-ppspp/pkg/ma"
)

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
	id                  []byte
	ledbat              *ledbat.Controller
	chunkRate           ma.Simple
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
func NewPeer(id []byte) *Peer {
	return &Peer{
		id:           id,
		ledbat:       ledbat.New(),
		chunkRate:    ma.NewSimple(500, 10*time.Millisecond),
		rttSampleBin: binmap.None,
		channels:     map[*Swarm]*channelWriter{},
	}
}

func (p *Peer) addChannel(s *Swarm, c *channelWriter) {
	p.Lock()
	defer p.Unlock()

	p.channels[s] = c
}

func (p *Peer) removeChannel(s *Swarm) *channelWriter {
	p.Lock()
	defer p.Unlock()

	c := p.channels[s]
	delete(p.channels, s)

	return c
}

func (p *Peer) addDelaySample(sample time.Duration, chunkSize int) {
	p.Lock()
	defer p.Unlock()

	p.ledbat.AddDelaySample(sample, chunkSize)
}

// outstandingChunks ...
func (p *Peer) outstandingChunks() int {
	return int(p.requestedChunkCount - (p.receivedChunkCount + p.cancelledChunkCount))
}

// addRequestedChunks ...
func (p *Peer) addRequestedChunks(n uint64) {
	p.requestedChunkCount += n
}

// addCancelledChunk ...
func (p *Peer) addCancelledChunk() {
	p.cancelledChunkCount++
}

func (p *Peer) addReceivedChunk() {
	p.Lock()
	defer p.Unlock()

	p.receivedChunkCount++
	p.chunkRate.AddWithTime(1, iotime.Load())
}

// chunkInterval ...
func (p *Peer) chunkInterval() time.Duration {
	return p.chunkRate.Interval()
}

// trackBinRTT ...
func (p *Peer) trackBinRTT(cid uint64, bin binmap.Bin, t time.Time) {
	if !p.rttSampleBin.IsNone() {
		return
	}

	p.rttSampleChannel = cid
	p.rttSampleBin = bin
	p.rttSampleTime = t
}

// trackPingRTT ...
func (p *Peer) trackPingRTT(cid uint64, t time.Time) (nonce uint64, ok bool) {
	if !p.rttSampleBin.IsNone() || t.Sub(p.rttSampleTime) < minPingInterval {
		return
	}

	// with even nonces Contains(nonce) is an equality check
	nonce = uint64(rand.Int63()) << 1

	p.rttSampleChannel = cid
	p.rttSampleBin = binmap.Bin(nonce)
	p.rttSampleTime = t

	return nonce, true
}

func (p *Peer) addRTTSample(cid uint64, b binmap.Bin, delay time.Duration) {
	p.Lock()
	defer p.Unlock()

	if p.rttSampleChannel != cid || !p.rttSampleBin.Contains(b) {
		return
	}

	rtt := iotime.Load().Sub(p.rttSampleTime)
	if rtt > delay {
		rtt -= delay
	}
	p.ledbat.AddRTTSample(rtt)
	p.rttSampleBin = binmap.None
}

// Close ...
func (p *Peer) Close() {
	// TODO: send empty handshake (ppspp goodbye)

	for s, c := range p.channels {
		s.removeChannel(p)
		c.Close()
	}
}
