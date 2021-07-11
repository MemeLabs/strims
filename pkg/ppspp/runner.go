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

const peerQOSUpdateInterval = time.Second
const minPeerQOSWeight = 50

// Conn ...
type Conn interface {
	ioutil.WriteFlushCloser
	MTU() int
	SetQOSWeight(w uint64)
}

type runnerSwarm struct {
	scheduler swarmScheduler
	stop      timeutil.StopFunc
	peers     map[*peer]codec.Channel
}

func NewRunner(ctx context.Context, logger *zap.Logger) *Runner {
	r := &Runner{
		logger: logger,
		swarms: map[*Swarm]*runnerSwarm{},
		peers:  map[*peer]*ChannelReader{},
	}
	timeutil.DefaultTickEmitter.Subscribe(r.run)
	return r
}

type Runner struct {
	logger *zap.Logger
	lock   sync.Mutex
	swarms map[*Swarm]*runnerSwarm
	peers  map[*peer]*ChannelReader

	nextPeerQOSUpdate timeutil.Time
}

// runSwarmPeer ...
func (r *Runner) runSwarmPeer(s *Swarm, p *peer, channel, peerChannel codec.Channel) error {
	r.lock.Lock()
	defer r.lock.Unlock()

	rs := r.swarms[s]
	if rs == nil {
		logger := r.logger.With(logutil.ByteHex("swarm", bytes.NewBuffer(s.id).Next(8)))
		ss := s.options.SchedulingMethod.swarmScheduler(logger, s)
		rs = &runnerSwarm{
			scheduler: ss,
			stop:      timeutil.DefaultTickEmitter.Subscribe(ss.Run),
			peers:     map[*peer]codec.Channel{},
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

func (r *Runner) stopSwarmPeer(s *Swarm, p *peer) {
	r.lock.Lock()
	defer r.lock.Unlock()

	rs := r.swarms[s]
	cr := r.peers[p]
	if rs == nil || cr == nil {
		return
	}

	c, ok := rs.peers[p]
	if !ok {
		return
	}
	r.stopChannel(s, p, rs, cr, c)
}

func (r *Runner) stopChannel(s *Swarm, p *peer, rs *runnerSwarm, cr *ChannelReader, c codec.Channel) {
	delete(rs.peers, p)
	if len(rs.peers) == 0 {
		rs.stop()
		delete(r.swarms, s)
	}

	cr.closeChannel(c)
	rs.scheduler.CloseChannel(p)

	deleteChannelWriterMetrics(s, p)
	deleteChannelReaderMetrics(s, p)
}

func (r *Runner) RunPeer(id []byte, w Conn) (*ChannelReader, *RunnerPeer) {
	p := newPeer(id, w, timeutil.DefaultTickEmitter.Ticker())
	cr := newChannelReader(r.logger.With(logutil.ByteHex("peer", id)))

	r.lock.Lock()
	r.peers[p] = cr
	r.lock.Unlock()

	return cr, &RunnerPeer{r, p}
}

func (r *Runner) tryStopPeer(p *peer) {
	r.lock.Lock()
	defer r.lock.Unlock()

	cr, ok := r.peers[p]
	if !ok {
		return
	}

	for s, rs := range r.swarms {
		if c, ok := rs.peers[p]; ok {
			r.stopChannel(s, p, rs, cr, c)
		}
	}

	delete(r.peers, p)
	p.close()
}

func (r *Runner) run(t timeutil.Time) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if t.After(r.nextPeerQOSUpdate) {
		r.nextPeerQOSUpdate = t.Add(peerQOSUpdateInterval)
		r.updatePeerWeights(t)
	}
}

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

		if weight < minPeerQOSWeight {
			weight = minPeerQOSWeight
		} else if weight > qos.MaxWeight {
			weight = qos.MaxWeight
		}

		p.w.SetQOSWeight(weight)
	}
}

// RunnerPeer ...
type RunnerPeer struct {
	r *Runner
	p *peer
}

// RunSwarm ...
func (p *RunnerPeer) RunSwarm(s *Swarm, channel, peerChannel codec.Channel) error {
	return p.r.runSwarmPeer(s, p.p, channel, peerChannel)
}

// StopSwarm ...
func (p *RunnerPeer) StopSwarm(s *Swarm) {
	p.r.stopSwarmPeer(s, p.p)
}

// Stop ...
func (p *RunnerPeer) Stop() {
	p.r.tryStopPeer(p.p)
}
