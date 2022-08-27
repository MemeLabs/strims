// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"log"
	"sync"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

type peerPriority uint8

func (p peerPriority) String() string {
	switch p {
	case peerPriorityHigh:
		return "HIGH"
	case peerPriorityLow:
		return "LOW"
	default:
		panic("invalid peer priority")
	}
}

const (
	peerPriorityHigh peerPriority = 0
	peerPriorityLow  peerPriority = 1
)

func newPeer(id []byte, w Conn, t timeutil.Ticker) *peer {
	p := &peer{
		id:     id,
		w:      w,
		ready:  make(chan timeutil.Time, 2),
		ticker: t,

		rq: newPeerTaskRunnerQueue(),

		m: newPeerMetrics(),
	}
	go p.run()
	return p
}

type peer struct {
	id []byte

	ready  chan timeutil.Time
	ticker timeutil.Ticker

	lock sync.Mutex
	w    Conn

	rq peerTaskRunnerQueue
	dq [2]peerDataQueue

	m  *peerMetrics
	sm syncutil.Map[*Swarm, *peerSwarmMetrics]
}

func (p *peer) ID() []byte {
	return p.id
}

func (p *peer) close() {
	close(p.ready)
	p.ticker.Stop()
}

func (p *peer) RemoveRunner(c peerTaskRunner) {
	p.lock.Lock()
	p.rq.Remove(c)
	p.dq[peerPriorityLow].Remove(c, binmap.All)
	p.dq[peerPriorityHigh].Remove(c, binmap.All)
	p.lock.Unlock()
}

func (p *peer) runAt(t timeutil.Time) {
	select {
	case p.ready <- t:
	default:
	}
}

func (p *peer) runNow() {
	p.runAt(timeutil.Now())
}

func (p *peer) enqueueAt(cs peerTaskRunner, t timeutil.Time) {
	p.lock.Lock()
	ok := p.rq.Push(cs)
	p.lock.Unlock()

	if ok || !t.IsNil() {
		p.runAt(t)
	}
}

func (p *peer) Enqueue(cs peerTaskRunner) {
	p.enqueueAt(cs, timeutil.NilTime)
}

func (p *peer) EnqueueNow(cs peerTaskRunner) {
	p.enqueueAt(cs, timeutil.Now())
}

func (p *peer) PushData(cs peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.dq[pri].Push(cs, b, t)
	p.lock.Unlock()
}

func (p *peer) PushFrontData(cs peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.dq[pri].PushFront(cs, b, t)
	p.lock.Unlock()
}

func (p *peer) RemoveData(cs peerTaskRunner, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.dq[pri].Remove(cs, b)
	p.lock.Unlock()
}

func (p *peer) run() {
	var t timeutil.Time
	for t = range p.ready {
		for t.IsNil() {
			var ok bool
			select {
			case t, ok = <-p.ticker.C:
			case t, ok = <-p.ready:
			}
			if !ok {
				return
			}
		}

		for {
			idle, err := p.write()
			if err != nil {
				log.Println(err)
				break
			}
			if idle {
				break
			}
		}
	}
}

func (p *peer) write() (bool, error) {
	p.lock.Lock()
	pws := p.rq.Detach()
	p.lock.Unlock()
	for {
		pw, ok := pws.Pop()
		if !ok {
			break
		}

		if _, err := pw.Write(); err != nil {
			if err != codec.ErrNotEnoughSpace {
				return true, err
			}

			p.lock.Lock()
			p.rq.Push(pw)
			p.rq.Reattach(pws)
			p.lock.Unlock()
			break
		}
	}

	for i := range p.dq {
		for {
			p.lock.Lock()
			pw, bin, t, ok := p.dq[i].Pop()
			p.lock.Unlock()
			if !ok {
				break
			}

			_, err := pw.WriteData(bin, t, peerPriority(i))
			if err == codec.ErrNotEnoughSpace {
				break
			}
		}
	}

	if err := p.w.Flush(); err != nil {
		return true, err
	}

	p.lock.Lock()
	idle := p.dq[peerPriorityLow].Empty() && p.dq[peerPriorityHigh].Empty() && p.rq.Empty()
	p.lock.Unlock()
	return idle, nil
}
