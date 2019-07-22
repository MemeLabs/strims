package encoding

import (
	"sync"
)

type ChannelsMap struct {
	lock     sync.Mutex
	channels map[uint32]*PeerChannel
}

func NewChannelsMap() *ChannelsMap {
	return &ChannelsMap{
		channels: map[uint32]*PeerChannel{},
	}
}

func (m *ChannelsMap) Get(id uint32) (c *PeerChannel, ok bool) {
	m.lock.Lock()
	defer m.lock.Unlock()
	c, ok = m.channels[id]
	return
}

func (m *ChannelsMap) Len() int {
	m.lock.Lock()
	defer m.lock.Unlock()
	return len(m.channels)
}

func (m *ChannelsMap) Insert(id uint32, c *PeerChannel) {
	m.lock.Lock()
	m.channels[id] = c
	m.lock.Unlock()
}

func (m *ChannelsMap) Delete(id uint32) {
	m.lock.Lock()
	delete(m.channels, id)
	m.lock.Unlock()
}

type Priority int

const (
	High Priority = iota
	Medium
	Low
	Skip
)

type Prioritizer interface {
	Prioritize(bin uint32) Priority
}

type MemePrioritizer struct {

}

func (p *MemePrioritizer) Prioritize(bin uint32) Priority {
	return High
}

func NewScheduler() *Scheduler {
	return &Scheduler{
		swarms: map[int]*Swarm{},
	}
}

type Scheduler struct {
	swarms map[int]*Swarm
}
