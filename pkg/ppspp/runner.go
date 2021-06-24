package ppspp

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"go.uber.org/zap"
)

// Conn ...
type Conn interface {
	ioutil.WriteFlushCloser
	MTU() int
	SetQOSWeight(w uint64)
}

type runnerSwarm struct {
	scheduler SwarmScheduler
	stop      timeutil.StopFunc
	peers     map[*Peer]codec.Channel
}

func NewRunner(ctx context.Context, logger *zap.Logger) *Runner {
	r := &Runner{
		logger: logger,
		swarms: map[*Swarm]*runnerSwarm{},
		peers:  map[*Peer]*ChannelReader{},
		ticker: timeutil.NewTickEmitter(100 * time.Millisecond),
	}
	r.ticker.Subscribe(r.run)
	return r
}

type Runner struct {
	logger *zap.Logger
	lock   sync.Mutex
	swarms map[*Swarm]*runnerSwarm
	peers  map[*Peer]*ChannelReader
	ticker *timeutil.TickEmitter

	nextPeerQOSUpdate timeutil.Time
}

// RunChannel ...
func (r *Runner) RunChannel(s *Swarm, p *Peer, channel, peerChannel codec.Channel) error {
	r.lock.Lock()
	defer r.lock.Unlock()

	rs := r.swarms[s]
	if rs == nil {
		logger := r.logger.With(logutil.ByteHex("swarm", bytes.NewBuffer(s.id).Next(8)))
		ss := s.options.SchedulingMethod.SwarmScheduler(logger, s)
		rs = &runnerSwarm{
			scheduler: ss,
			stop:      r.ticker.Subscribe(ss.Run),
			peers:     map[*Peer]codec.Channel{},
		}
		s.pubSub.Subscribe(ss)
		r.swarms[s] = rs
	}

	cr := r.peers[p]
	if cr == nil {
		return errors.New("channel cannot be run with closed peer")
	}

	if _, ok := rs.peers[p]; ok {
		return errors.New("channel for swarm/peer pair already running")
	}
	rs.peers[p] = channel

	cw := newChannelWriter(newChannelWriterMetrics(s, p), p.w, peerChannel)
	cs := rs.scheduler.ChannelScheduler(p, cw)
	cr.openChannel(channel, newChannelReaderMetrics(s, p), cs, s)

	if err := cs.WriteHandshake(); err != nil {
		return err
	}
	if err := cw.Flush(); err != nil {
		return err
	}
	if err := p.w.Flush(); err != nil {
		return err
	}

	return nil
}

// StopChannel ...
func (r *Runner) StopChannel(s *Swarm, p *Peer) {
	r.lock.Lock()
	defer r.lock.Unlock()
	r.stopChannel(s, p)
}

func (r *Runner) stopChannel(s *Swarm, p *Peer) {
	rs := r.swarms[s]
	cr := r.peers[p]
	if rs == nil || cr == nil {
		return
	}

	channel, ok := rs.peers[p]
	if !ok {
		return
	}

	delete(rs.peers, p)
	if len(rs.peers) == 0 {
		rs.stop()
		delete(r.swarms, s)
	}

	cr.closeChannel(channel)
	rs.scheduler.CloseChannel(p)

	deleteChannelWriterMetrics(s, p)
	deleteChannelReaderMetrics(s, p)
}

func (r *Runner) RunPeer(id []byte, w Conn) (*ChannelReader, *Peer) {
	p := newPeer(id, w, r.ticker.Ticker())
	cr := newChannelReader(r.logger.With(logutil.ByteHex("peer", id)))

	r.lock.Lock()
	r.peers[p] = cr
	r.lock.Unlock()

	return cr, p
}

func (r *Runner) StopPeer(p *Peer) {
	r.lock.Lock()
	defer r.lock.Unlock()

	for s, rs := range r.swarms {
		if _, ok := rs.peers[p]; ok {
			r.stopChannel(s, p)
		}
	}

	delete(r.peers, p)
	p.close()
}

func (r *Runner) run(t timeutil.Time) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if t.After(r.nextPeerQOSUpdate) {
		r.updatePeerWeights(t)
		r.nextPeerQOSUpdate = t.Add(PeerQOSUpdateInterval)
	}
}

const PeerQOSUpdateInterval = time.Second
const MinPeerQOSWeight = 50

func (r *Runner) updatePeerWeights(t timeutil.Time) {
	var totalBytes uint64
	for p := range r.peers {
		p.lock.Lock()
		totalBytes += p.receivedBytes.RateWithTime(time.Minute, t)
		p.lock.Unlock()
	}
	if totalBytes == 0 {
		return
	}

	for p := range r.peers {
		p.lock.Lock()
		weight := p.receivedBytes.RateWithTime(time.Minute, t) * qos.MaxWeight / totalBytes
		p.lock.Unlock()

		if weight < MinPeerQOSWeight {
			weight = MinPeerQOSWeight
		} else if weight > qos.MaxWeight {
			weight = qos.MaxWeight
		}

		p.w.SetQOSWeight(weight)
	}
}
