package ppspp

import (
	"errors"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/stats"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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

		receivedBytes: stats.NewSMA(60, time.Second),

		wq: newPeerWriterQueue(),
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

	receivedBytes stats.SMA

	wq peerWriterQueue
	ds [2]peerDataQueue
}

func (p *peer) ID() []byte {
	return p.id
}

func (p *peer) close() {
	close(p.ready)
	p.ticker.Stop()
	p.w.Close()
}

func (p *peer) CloseChannel(c peerWriter) {
	p.lock.Lock()
	p.wq.Remove(c)
	p.ds[peerPriorityLow].Remove(c, binmap.All)
	p.ds[peerPriorityHigh].Remove(c, binmap.All)
	p.lock.Unlock()
}

func (p *peer) AddReceivedBytes(n uint64, t timeutil.Time) {
	p.lock.Lock()
	p.receivedBytes.AddWithTime(n, t)
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

func (p *peer) enqueueAt(cs peerWriter, t timeutil.Time) {
	p.lock.Lock()
	ok := p.wq.Push(cs)
	p.lock.Unlock()

	if ok || !t.IsNil() {
		p.runAt(t)
	}
}

func (p *peer) Enqueue(cs peerWriter) {
	p.enqueueAt(cs, timeutil.NilTime)
}

func (p *peer) EnqueueNow(cs peerWriter) {
	p.enqueueAt(cs, timeutil.Now())
}

func (p *peer) PushData(cs peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Push(cs, b, t)
	p.lock.Unlock()
}

func (p *peer) PushFrontData(cs peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].PushFront(cs, b, t)
	p.lock.Unlock()
}

func (p *peer) RemoveData(cs peerWriter, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Remove(cs, b)
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
	var n int

	p.lock.Lock()
	pws := p.wq.Detach()
	p.lock.Unlock()
	for {
		pw, ok := pws.Pop()
		if !ok {
			break
		}

		nn, err := pw.Write(p.w.MTU() - n)
		n += nn
		if err != nil {
			if !errors.Is(err, codec.ErrNotEnoughSpace) {
				return true, err
			}

			p.lock.Lock()
			p.wq.Push(pw)
			p.wq.Reattach(pws)
			p.lock.Unlock()
			break
		}
	}

	for i := range p.ds {
		for {
			p.lock.Lock()
			pw, bin, t, ok := p.ds[i].Pop()
			p.lock.Unlock()
			if !ok {
				break
			}

			nn, err := pw.WriteData(p.w.MTU()-n, bin, t, peerPriority(i))
			n += nn
			if errors.Is(err, codec.ErrNotEnoughSpace) {
				break
			}
		}
	}

	if err := p.w.Flush(); err != nil {
		return true, err
	}

	p.lock.Lock()
	idle := p.ds[peerPriorityLow].Empty() && p.ds[peerPriorityHigh].Empty() && p.wq.Empty()
	p.lock.Unlock()
	return idle, nil
}
