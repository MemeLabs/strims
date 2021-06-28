package ppspp

import (
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
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

func newPeer(id []byte, w Conn, t timeutil.Ticker) *Peer {
	p := &Peer{
		id:     id,
		w:      w,
		ready:  make(chan timeutil.Time, 1),
		ticker: t,

		receivedBytes: stats.NewSMA(60, time.Second),

		wq: newPeerWriterQueue(),
	}
	go p.run()
	return p
}

type Peer struct {
	id []byte

	ready  chan timeutil.Time
	ticker timeutil.Ticker

	lock sync.Mutex
	w    Conn

	receivedBytes stats.SMA

	wq peerWriterQueue
	ds [2]peerDataQueue
}

func (p *Peer) ID() []byte {
	return p.id
}

func (p *Peer) close() {
	close(p.ready)
	p.ticker.Stop()
	p.w.Close()
}

func (p *Peer) closeChannel(c PeerWriter) {
	p.lock.Lock()
	p.wq.Remove(c)
	p.ds[peerPriorityLow].Remove(c, binmap.All)
	p.ds[peerPriorityHigh].Remove(c, binmap.All)
	p.lock.Unlock()
}

func (p *Peer) addReceivedBytes(n uint64, t timeutil.Time) {
	p.lock.Lock()
	p.receivedBytes.AddWithTime(n, t)
	p.lock.Unlock()
}

func (p *Peer) runAt(t timeutil.Time) {
	select {
	case p.ready <- t:
	default:
	}
}

func (p *Peer) runNow() {
	p.runAt(timeutil.Now())
}

func (p *Peer) enqueueAt(cs PeerWriter, t timeutil.Time) {
	p.lock.Lock()
	ok := p.wq.Push(cs)
	p.lock.Unlock()

	if ok || !t.IsNil() {
		p.runAt(t)
	}
}

func (p *Peer) enqueue(cs PeerWriter) {
	p.enqueueAt(cs, timeutil.NilTime)
}

func (p *Peer) enqueueNow(cs PeerWriter) {
	p.enqueueAt(cs, timeutil.Now())
}

func (p *Peer) pushData(cs PeerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Push(cs, b, t)
	p.lock.Unlock()
}

func (p *Peer) pushFrontData(cs PeerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].PushFront(cs, b, t)
	p.lock.Unlock()
}

func (p *Peer) removeData(cs PeerWriter, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Remove(cs, b)
	p.lock.Unlock()
}

func (p *Peer) run() {
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

func (p *Peer) write() (bool, error) {
	var n int

	p.lock.Lock()
	cs := p.wq.Detach()
	p.lock.Unlock()
	for {
		c, ok := cs.Pop()
		if !ok {
			break
		}

		nn, err := c.Write(p.w.MTU() - n)
		if err == nil {
			n += nn
		}
	}

	for i := range p.ds {
		for {
			p.lock.Lock()
			cs, bin, t, ok := p.ds[i].Pop()
			p.lock.Unlock()
			if !ok {
				break
			}

			nn, err := cs.WriteData(p.w.MTU()-n, bin, t, peerPriority(i))
			if err != nil {
				return true, err
			}
			if nn == 0 {
				break
			}
			n += nn
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
