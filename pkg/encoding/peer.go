package encoding

import (
	"encoding/json"
	"fmt"
	"io"
	"math/rand"
	"path"
	"runtime"
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

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.WriteCloser
	MTU() int
	Flush() error
}

const minPingInterval = time.Second * 10

// Peer ...
type Peer struct {
	sync.Mutex

	// id                  int64
	channels            channels
	lastActive          int64
	ledbat              *ledbat.Controller
	chunkIntervalMean   ma.Simple
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
func NewPeer() *Peer {
	return &Peer{
		lastActive:        iotime.Load().Unix(),
		ledbat:            ledbat.New(),
		chunkIntervalMean: ma.NewSimple(500, 10*time.Millisecond),
		rttSampleBin:      binmap.None,
	}
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
	p.chunkIntervalMean.Add(1)
}

// TrackBinRTT ...
// func (p *Peer) TrackBinRTT(cid uint32, b binmap.Bin) (ok bool) {
// 	if ok = p.rttSampleBin.IsNone(); ok {
// 		p.rttSampleChannel = cid
// 		p.rttSampleBin = b
// 		p.rttSampleTime = time.Now()
// 	}
// 	return
// }

// TrackPingRTT ...
func (p *Peer) TrackPingRTT(t time.Time) (nonce uint64, ok bool) {
	if ok = t.Sub(p.rttSampleTime) > minPingInterval; ok {
		// with even nonces Contains(nonce) is an equality check
		nonce = uint64(rand.Int63()) << 1

		p.rttSampleChannel = 0
		p.rttSampleBin = binmap.Bin(nonce)
		p.rttSampleTime = t
	}
	return
}

// AddRTTSample ...
func (p *Peer) AddRTTSample(cid uint32, b binmap.Bin) {
	if p.rttSampleChannel == cid && p.rttSampleBin.Contains(b) {
		p.ledbat.AddRTTSample(iotime.Load().Sub(p.rttSampleTime))
		p.rttSampleBin = binmap.None
	}
}

// ChunkIntervalMean ...
func (p *Peer) ChunkIntervalMean() time.Duration {
	return p.chunkIntervalMean.Interval()
}

// Close ...
func (p *Peer) Close() {
	// TODO: send empty handshake (ppspp goodbye)
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}
