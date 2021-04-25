package ppspp

import (
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ma"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type peerPriority uint8

const (
	peerPriorityHigh peerPriority = 0
	peerPriorityLow  peerPriority = 1
)

type PeerWriter interface {
	Write(maxBytes int) (int, error)
	WriteData(maxBytes int, b binmap.Bin, pri peerPriority) (int, error)
}

func newPeer(id []byte, w Conn, t timeutil.Ticker) *Peer {
	p := &Peer{
		id:     id,
		w:      w,
		ready:  make(chan time.Time, 1),
		ticker: t,

		receivedBytes: ma.NewSimple(60, time.Second),
		sentBytes:     ma.NewSimple(60, time.Second),

		cs: newPeerWriterQueue(),
	}
	go p.run()
	return p
}

type Peer struct {
	id []byte

	ready  chan time.Time
	ticker timeutil.Ticker

	lock sync.Mutex
	w    Conn

	receivedBytes ma.Simple
	sentBytes     ma.Simple

	cs peerWriterQueue
	ds [2]peerDataQueue
}

func (p *Peer) close() {
	close(p.ready)
	p.ticker.Stop()
	p.w.Close()
}

func (p *Peer) closeChannel(c PeerWriter) {
	p.lock.Lock()
	p.cs.Remove(c)
	p.ds[peerPriorityLow].Remove(c, binmap.All)
	p.ds[peerPriorityHigh].Remove(c, binmap.All)
	p.lock.Unlock()
}

func (p *Peer) runAt(t time.Time) {
	select {
	case p.ready <- t:
	default:
	}
}

func (p *Peer) runNow() {
	p.runAt(iotime.Load())
}

func (p *Peer) enqueueAt(qt *PeerWriterQueueTicket, cs PeerWriter, t time.Time) {
	p.lock.Lock()
	ok := p.cs.Push(qt, cs)
	p.lock.Unlock()

	if ok || !t.IsZero() {
		p.runAt(t)
	}
}

func (p *Peer) enqueue(qt *PeerWriterQueueTicket, cs PeerWriter) {
	p.enqueueAt(qt, cs, time.Time{})
}

func (p *Peer) enqueueNow(qt *PeerWriterQueueTicket, cs PeerWriter) {
	p.enqueueAt(qt, cs, iotime.Load())
}

func (p *Peer) pushData(cs ChannelScheduler, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Push(cs, b)
	p.lock.Unlock()
}

func (p *Peer) pushFrontData(cs ChannelScheduler, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].PushFront(cs, b)
	p.lock.Unlock()
}

func (p *Peer) removeData(cs ChannelScheduler, b binmap.Bin, pri peerPriority) {
	p.lock.Lock()
	p.ds[pri].Remove(cs, b)
	p.lock.Unlock()
}

func (p *Peer) run() {
	var t time.Time
	for t = range p.ready {
		for t.IsZero() {
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
	cs := p.cs.Detach()
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
			cs, bin, ok := p.ds[i].Pop()
			p.lock.Unlock()
			if !ok {
				break
			}

			nn, err := cs.WriteData(p.w.MTU()-n, bin, peerPriority(i))
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
	idle := p.ds[peerPriorityLow].Empty() && p.ds[peerPriorityHigh].Empty() && p.cs.Empty()
	p.lock.Unlock()
	return idle, nil
}
