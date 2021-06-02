package store

import (
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// NewPubSub ...
func NewPubSub(subs ...Subscriber) *PubSub {
	return &PubSub{
		subs: subs,
	}
}

// PubSub ...
type PubSub struct {
	lock sync.Mutex
	subs []Subscriber
}

// Subscribe ...
func (p *PubSub) Subscribe(s Subscriber) {
	p.lock.Lock()
	defer p.lock.Unlock()
	p.subs = append(p.subs, s)
}

// Unsubscribe ...
func (p *PubSub) Unsubscribe(s Subscriber) {
	p.lock.Lock()
	defer p.lock.Unlock()

	for i := 0; i < len(p.subs); i++ {
		if p.subs[i] == s {
			copy(p.subs[i:], p.subs[i+1:])
			p.subs = p.subs[:len(p.subs)-1]
		}
	}
}

// Publish ...
func (p *PubSub) Publish(c Chunk) {
	p.lock.Lock()
	defer p.lock.Unlock()

	for _, s := range p.subs {
		s.Consume(c)
	}
}

// Subscriber ...
type Subscriber interface {
	Consume(c Chunk)
}

// Chunk ...
type Chunk struct {
	Bin  binmap.Bin
	Data []byte
}
