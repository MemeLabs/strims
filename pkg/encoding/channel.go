package encoding

import (
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// channelOptions ...
type channelOptions struct {
	ID    uint32
	Swarm *Swarm
	Peer  *Peer
	Conn  TransportConn
}

// newchannel ...
func newChannel(o channelOptions) *channel {
	return &channel{
		id:                  o.ID,
		swarm:               o.Swarm,
		peer:                o.Peer,
		conn:                o.Conn,
		addedBins:           binmap.New(),
		requestedBins:       binmap.New(),
		availableBins:       binmap.New(),
		unackedBins:         binmap.New(),
		sentBinHistory:      newBinHistory(32),
		requestedBinHistory: newBinHistory(32),
		acks:                []Ack{},
	}
}

// channel ...
type channel struct {
	sync.Mutex

	id       uint32
	remoteID uint32
	swarm    *Swarm
	peer     *Peer
	conn     TransportConn

	handshakeSent bool
	choked        bool

	addedBins           *binmap.Map // bins to send HAVEs for
	requestedBins       *binmap.Map // bins to send DATA for
	availableBins       *binmap.Map // bins the peer claims to have
	unackedBins         *binmap.Map // sent bins that have not been acked
	sentBinHistory      *binHistory // recently sent bins
	requestedBinHistory *binHistory // bins recently requested from the peer
	acks                []Ack
}

func (c *channel) Swarm() *Swarm {
	return c.swarm
}

func (c *channel) Peer() *Peer {
	return c.peer
}

func (c *channel) Close() {
	c.swarm.Lock()
	defer c.swarm.Unlock()
}

// OfferHandshake ...
func (c *channel) OfferHandshake(w *MemeWriter) {
	log.Println("offering handshake")
	c.handshakeSent = true

	swarmID := SwarmIdentifierProtocolOption(c.swarm.ID.Binary())
	w.Write(&Handshake{
		ChannelID: c.id,
		Options: []ProtocolOption{
			&VersionProtocolOption{Value: 2},
			&MinimumVersionProtocolOption{Value: 2},
			&swarmID,
		},
	})
}

// HandleMemeRequest ...
func (c *channel) HandleMemeRequest(w *MemeWriter, r *MemeRequest) {
	c.peer.UpdateLastActive()
	w.BeginFrame(c.remoteID)

	// spew.Dump(r)

	for _, mi := range r.Messages {
		switch m := mi.(type) {
		case *Handshake:
			c.handleHandshake(w, m)
		case *Data:
			c.handleData(w, m)
		case *Ack:
			c.handleAck(w, m)
		case *Have:
			c.handleHave(w, m)
		case *Request:
			c.handleRequest(w, m)
		case *Cancel:
			c.handleCancel(w, m)
		case *Choke:
			c.handleChoke(w, m)
		case *Unchoke:
			c.handleUnchoke(w, m)
		case *Ping:
			c.handlePing(w, m)
		case *Pong:
			c.handlePong(w, m)
		}
	}
}

func (c *channel) handleHandshake(w *MemeWriter, v *Handshake) {
	c.Lock()
	defer c.Unlock()

	log.Println("received handshake")

	cid := v.ChannelID
	c.remoteID = cid

	// TODO: set live discard window
	// TODO: verify protocol options
	// TODO: close channels on empty handshake

	w.BeginFrame(cid)

	if c.handshakeSent {
		// TODO: send PEX REQ here
		return
	}
	c.handshakeSent = true

	log.Println("sending handshake")
	w.Write(&Handshake{
		ChannelID: c.id,
		Options:   v.Options,
	})

	c.swarm.Lock()
	defer c.swarm.Unlock()

	b := c.swarm.loadedBins.FindFilled()
	for {
		b = c.swarm.loadedBins.Cover(b)
		w.Write(&Have{Address(b)})

		b = c.swarm.loadedBins.FindFilledAfter(b.BaseRight() + 2)
		if b.IsNone() {
			break
		}
	}
}

func (c *channel) handleData(w *MemeWriter, v *Data) {
	c.Lock()
	c.acks = append(c.acks, Ack{
		Address:     v.Address,
		DelaySample: v.Timestamp,
	})
	c.Unlock()

	c.swarm.WriteChunk(v.Address.Bin(), v.Data)

	c.peer.Lock()
	c.peer.AddReceivedChunk()
	c.peer.Unlock()
}

func (c *channel) handleAck(w *MemeWriter, v *Ack) {
	b := v.Address.Bin()

	c.Lock()
	if !c.unackedBins.FilledAt(b) {
		c.Unlock()
		return
	}
	c.availableBins.Set(b)
	c.unackedBins.Reset(b)
	c.Unlock()

	c.peer.Lock()
	c.peer.ledbat.AddDelaySample(time.Now().Sub(v.DelaySample.Time), ChunkSize)
	c.peer.AddRTTSample(c.id, b)
	c.peer.AddAckedChunk()
	c.peer.Unlock()
}

func (c *channel) handleHave(w *MemeWriter, v *Have) {
	c.Lock()
	defer c.Unlock()
	c.availableBins.Set(v.Bin())
}

func (c *channel) handleRequest(w *MemeWriter, v *Request) {
	c.Lock()
	defer c.Unlock()
	c.requestedBins.Set(v.Bin())
}

func (c *channel) handleCancel(w *MemeWriter, v *Cancel) {
	c.peer.Lock()
	c.Lock()

	b := v.Address.Bin()
	if !c.unackedBins.EmptyAt(b) {
		// TODO: this isn't accurate if the bin was partially acked
		c.peer.ledbat.AddDataLoss(int(b.BaseLeft())*ChunkSize, false)
		c.unackedBins.Reset(b)
	}
	c.requestedBins.Reset(b)

	c.Unlock()
	c.peer.Unlock()
}

func (c *channel) handleChoke(w *MemeWriter, v *Choke) {
	c.Lock()
	defer c.Unlock()
	c.choked = true
}

func (c *channel) handleUnchoke(w *MemeWriter, v *Unchoke) {
	c.Lock()
	defer c.Unlock()
	c.choked = false
}

func (c *channel) handlePing(w *MemeWriter, v *Ping) {
	w.Write(&Pong{v.Nonce})
}

func (c *channel) handlePong(w *MemeWriter, v *Pong) {
	c.peer.Lock()
	c.peer.AddRTTSample(0, binmap.Bin(v.Nonce.Value))
	c.peer.Unlock()
}
