package encoding

import (
	"sync/atomic"
	"time"
)

type Peer struct {
	UID        int64
	channels   *ChannelsMap
	lastActive int64
}

func NewPeer(uid int64) *Peer {
	return &Peer{
		UID:        uid,
		channels:   NewChannelsMap(),
		lastActive: time.Now().Unix(),
	}
}

func (p *Peer) UpdateLastActive() {
	atomic.StoreInt64(&p.lastActive, time.Now().Unix())
}

func (p *Peer) LastActive() int64 {
	return atomic.LoadInt64(&p.lastActive)
}

type PeerChannel struct {
	ID       uint32
	RemoteID uint32
	s        *Swarm
	p        *Peer
	t        Transport
	c        TransportConn
}

func (c *PeerChannel) HandleMemeRequest(w MemeWriter, r *MemeRequest) {
	w.SetChannelID(c.RemoteID)

	for _, mi := range r.Messages {
		switch m := mi.(type) {
		case *Handshake:
			c.HandleHandshake(w, m)
		case *Data:
			c.HandleData(w, m)
		case *Ack:
			c.HandleAck(w, m)
		case *Have:
			c.HandleHave(w, m)
		case *Request:
			c.HandleRequest(w, m)
		case *Cancel:
			c.HandleCancel(w, m)
		case *Choke:
			c.HandleChoke(w, m)
		case *Unchoke:
			c.HandleUnchoke(w, m)
		}
	}
}

func (c *PeerChannel) HandleHandshake(w MemeWriter, v *Handshake) {
	w.Write(&Handshake{
		ChannelID: c.ID,
		Options:   v.Options,
	})
}

func (c *PeerChannel) HandleData(w MemeWriter, v *Data) {
	w.Write(&Ack{
		Address:     v.Address,
		DelaySample: v.Timestamp,
	})
}

func (c *PeerChannel) HandleAck(w MemeWriter, v *Ack) {
	return
}

func (c *PeerChannel) HandleHave(w MemeWriter, v *Have) {
	return
}

func (c *PeerChannel) HandleRequest(w MemeWriter, v *Request) {
	return
}

func (c *PeerChannel) HandleCancel(w MemeWriter, v *Cancel) {
	return
}

func (c *PeerChannel) HandleChoke(w MemeWriter, v *Choke) {
	return
}

func (c *PeerChannel) HandleUnchoke(w MemeWriter, v *Unchoke) {
	return
}
