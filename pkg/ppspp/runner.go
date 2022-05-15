// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/stats"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"go.uber.org/zap"
)

const peerQOSUpdateInterval = time.Second
const minPeerQOSWeight = 50

// Conn ...
type Conn interface {
	ioutil.BufferedWriteFlusher
	SetQOSWeight(w uint64)
}

type runnerSwarm struct {
	scheduler  swarmScheduler
	stopTicker timeutil.StopFunc
	peers      map[*peer]codec.Channel
}

func NewRunner(ctx context.Context, logger *zap.Logger) *Runner {
	r := &Runner{
		logger: logger,
		swarms: map[*Swarm]*runnerSwarm{},
		peers:  map[*peer]*ChannelReader{},
	}
	timeutil.DefaultTickEmitter.SubscribeCtx(ctx, peerQOSUpdateInterval, r.updatePeerWeights, nil)
	return r
}

type Runner struct {
	logger *zap.Logger
	lock   sync.Mutex
	swarms map[*Swarm]*runnerSwarm
	peers  map[*peer]*ChannelReader
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
			scheduler:  ss,
			stopTicker: timeutil.DefaultTickEmitter.DefaultSubscribe(ss.Run),
			peers:      map[*peer]codec.Channel{},
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

	sm := newPeerSwarmMetrics(p.m)
	p.sm.Set(s, sm)

	cwm := newPeerChannelWriterMetrics(p.m, sm)
	crm := newPeerChannelReaderMetrics(p.m, sm)

	cw := newChannelWriter(newChannelWriterMetrics(s, p, cwm), p.w, peerChannel)
	cs := rs.scheduler.ChannelScheduler(p, cw)
	cr.openChannel(channel, newChannelReaderMetrics(s, p, crm), cs, s)

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

	p.sm.Delete(s)
}

func (r *Runner) stopChannel(s *Swarm, p *peer, rs *runnerSwarm, cr *ChannelReader, c codec.Channel) {
	delete(rs.peers, p)
	if len(rs.peers) == 0 {
		rs.stopTicker()
		delete(r.swarms, s)
	}

	cr.closeChannel(c)
	rs.scheduler.CloseChannel(p)

	deleteChannelWriterMetrics(s, p)
	deleteChannelReaderMetrics(s, p)
}

func (r *Runner) RunPeer(id []byte, w Conn) (*ChannelReader, *RunnerPeer) {
	p := newPeer(id, w, timeutil.DefaultTickEmitter.DefaultTicker())
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

func (r *Runner) updatePeerWeights(t timeutil.Time) {
	r.lock.Lock()
	defer r.lock.Unlock()

	var totalBytes uint64
	for p := range r.peers {
		totalBytes += p.m.ReadDataBytesRate(t)
	}
	if totalBytes == 0 {
		return
	}

	for p := range r.peers {
		weight := p.m.ReadDataBytesRate(t) * qos.MaxWeight / totalBytes

		if weight < minPeerQOSWeight {
			weight = minPeerQOSWeight
		} else if weight > qos.MaxWeight {
			weight = qos.MaxWeight
		}

		p.w.SetQOSWeight(weight)
	}
}

type PeerMetricsSnapshot struct {
	MetricsSnapshot
	Swarms map[*Swarm]MetricsSnapshot
}

type MetricsSnapshot struct {
	Read  TransferMetricsSnapshot
	Write TransferMetricsSnapshot
}

type TransferMetricsSnapshot struct {
	Count uint64
	Rate  uint64
}

func newTransferMetrics() transferMetrics {
	return transferMetrics{
		bytesRate: stats.NewSMA(60, time.Second),
	}
}

type transferMetrics struct {
	bytesCount uint64
	bytesRate  stats.SMA
}

func (m *transferMetrics) AddDataBytesCount(n uint64) {
	m.bytesCount += n
	m.bytesRate.Add(n)
}

func (m *transferMetrics) Snapshot(t timeutil.Time) TransferMetricsSnapshot {
	return TransferMetricsSnapshot{
		Count: m.bytesCount,
		Rate:  m.bytesRate.RateWithTime(time.Second, t),
	}
}

func newPeerMetrics() *peerMetrics {
	return &peerMetrics{
		reader: newTransferMetrics(),
		writer: newTransferMetrics(),
	}
}

type peerMetrics struct {
	lock   sync.Mutex
	reader transferMetrics
	writer transferMetrics
}

func (m *peerMetrics) ReadDataBytesRate(t timeutil.Time) uint64 {
	m.lock.Lock()
	defer m.lock.Unlock()
	return m.reader.bytesRate.RateWithTime(time.Second, t)
}

func (m *peerMetrics) Snapshot(t timeutil.Time) MetricsSnapshot {
	m.lock.Lock()
	defer m.lock.Unlock()
	return MetricsSnapshot{
		Read:  m.reader.Snapshot(t),
		Write: m.writer.Snapshot(t),
	}
}

func newPeerSwarmMetrics(pm *peerMetrics) *peerSwarmMetrics {
	return &peerSwarmMetrics{
		peerMetrics: pm,
		reader:      newTransferMetrics(),
		writer:      newTransferMetrics(),
	}
}

type peerSwarmMetrics struct {
	*peerMetrics
	reader transferMetrics
	writer transferMetrics
}

func (m *peerSwarmMetrics) Snapshot(t timeutil.Time) MetricsSnapshot {
	m.peerMetrics.lock.Lock()
	defer m.peerMetrics.lock.Unlock()
	return MetricsSnapshot{
		Read:  m.reader.Snapshot(t),
		Write: m.writer.Snapshot(t),
	}
}

func newPeerChannelReaderMetrics(pm *peerMetrics, sm *peerSwarmMetrics) *peerChannelMetrics {
	return &peerChannelMetrics{
		peerMetrics: pm,
		peer:        &pm.reader,
		channel:     &sm.reader,
	}
}

func newPeerChannelWriterMetrics(pm *peerMetrics, sm *peerSwarmMetrics) *peerChannelMetrics {
	return &peerChannelMetrics{
		peerMetrics: pm,
		peer:        &pm.writer,
		channel:     &sm.writer,
	}
}

type peerChannelMetrics struct {
	*peerMetrics
	peer    *transferMetrics
	channel *transferMetrics
}

func (p *peerChannelMetrics) AddDataBytesCount(b uint64) {
	p.lock.Lock()
	defer p.lock.Unlock()
	p.peer.AddDataBytesCount(b)
	p.channel.AddDataBytesCount(b)
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

func (p *RunnerPeer) MetricsSnapshot(t timeutil.Time) PeerMetricsSnapshot {
	swarms := map[*Swarm]MetricsSnapshot{}
	p.p.sm.Each(func(s *Swarm, m *peerSwarmMetrics) {
		swarms[s] = m.Snapshot(t)
	})

	return PeerMetricsSnapshot{
		MetricsSnapshot: p.p.m.Snapshot(t),
		Swarms:          swarms,
	}
}
