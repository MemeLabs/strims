package encoding

import (
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var channelMessageCount = promauto.NewCounterVec(prometheus.CounterOpts{
	Name: "strims_ppspp_message_count",
	Help: "The total number of ppspp messages read per channel",
}, []string{"channel_id", "type"})

// Channel ...
// TODO: do we still need this?
type Channel interface {
	Write(b []byte) (int, error)
	Close()
	Done() <-chan struct{}
}

// channelOptions ...
type channelOptions struct {
	ID    uint32
	Swarm *Swarm
	Peer  *Peer
	Conn  ReadWriteFlusher
}

var nextChannelID uint32

// newchannel ...
func newChannel(o channelOptions) *channel {
	id := atomic.AddUint32(&nextChannelID, 1)

	return &channel{
		id:                  id,
		swarm:               o.Swarm,
		peer:                o.Peer,
		w:                   newDatagramWriter(o.Conn, o.Conn.MTU()),
		addedBins:           binmap.New(),
		requestedBins:       binmap.New(),
		availableBins:       binmap.New(),
		unackedBins:         binmap.New(),
		sentBinHistory:      newBinHistory(32),
		requestedBinHistory: newBinHistory(32),
		acks:                []Ack{},
		done:                make(chan struct{}),
		metrics:             newChannelMetrics(id),
	}
}

// channel ...
type channel struct {
	sync.Mutex

	id       uint32
	remoteID uint32
	swarm    *Swarm
	peer     *Peer
	conn     ReadWriteFlusher
	w        *datagramWriter

	handshakeSent bool
	choked        bool

	addedBins           *binmap.Map // bins to send HAVEs for
	requestedBins       *binmap.Map // bins to send DATA for
	availableBins       *binmap.Map // bins the peer claims to have
	unackedBins         *binmap.Map // sent bins that have not been acked
	sentBinHistory      *binHistory // recently sent bins
	requestedBinHistory *binHistory // bins recently requested from the peer
	acks                []Ack
	peerRequest         bool
	peerRequestTime     time.Time
	sentPeerRequestTime time.Time

	closeOnce sync.Once
	done      chan struct{}

	metrics channelMetrics
}

func (c *channel) Swarm() *Swarm {
	return c.swarm
}

func (c *channel) Peer() *Peer {
	return c.peer
}

func (c *channel) Write(b []byte) (n int, err error) {
	n, err = datagramReader{c}.Read(b)

	c.w.Flush()

	return
}

func (c *channel) Close() {
	c.closeOnce.Do(func() {
		close(c.done)
		c.metrics.Delete()
	})
}

func (c *channel) Done() <-chan struct{} {
	return c.done
}

// OfferHandshake ...
func (c *channel) OfferHandshake() {
	c.handshakeSent = true

	swarmID := SwarmIdentifierProtocolOption(c.swarm.ID.Binary())
	c.w.Write(&Handshake{
		ChannelID: c.id,
		Options: []ProtocolOption{
			&VersionProtocolOption{Value: 2},
			&MinimumVersionProtocolOption{Value: 2},
			&swarmID,
		},
	})
	c.w.Flush()
}

func (c *channel) HandleHandshake(v Handshake) {
	c.metrics.HandshakeCount.Inc()

	c.Lock()
	defer c.Unlock()

	c.swarm.Lock()
	defer c.swarm.Unlock()

	b := c.swarm.chunks.bins.FindFilled()
	for {
		b = c.swarm.chunks.bins.Cover(b)
		c.w.Write(&Have{Address(b)})

		b = c.swarm.chunks.bins.FindFilledAfter(b.BaseRight() + 2)
		if b.IsNone() {
			break
		}
	}
}

func (c *channel) HandleData(v Data) {
	c.metrics.DataCount.Inc()

	c.Lock()
	c.acks = append(c.acks, Ack{
		Address:     v.Address,
		DelaySample: DelaySample{iotime.Load().Sub(v.Timestamp.Time)},
	})
	c.Unlock()

	c.swarm.WriteChunk(v.Address.Bin(), v.Data)

	c.peer.Lock()
	c.peer.AddReceivedChunk()
	c.peer.Unlock()
}

func (c *channel) HandleAck(v Ack) {
	c.metrics.AckCount.Inc()

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
	c.peer.ledbat.AddDelaySample(v.DelaySample.Duration, ChunkSize)
	c.peer.AddAckedChunk()
	c.peer.Unlock()
}

func (c *channel) HandleHave(v Have) {
	c.metrics.HaveCount.Inc()

	c.Lock()
	defer c.Unlock()
	c.availableBins.Set(v.Bin())
}

func (c *channel) HandleRequest(v Request) {
	c.metrics.RequestCount.Inc()

	c.Lock()
	defer c.Unlock()
	c.requestedBins.Set(v.Bin())
}

func (c *channel) HandleCancel(v Cancel) {
	c.metrics.CancelCount.Inc()

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

func (c *channel) HandleChoke(v Choke) {
	c.metrics.ChokeCount.Inc()

	c.Lock()
	defer c.Unlock()
	c.choked = true
}

func (c *channel) HandleUnchoke(v Unchoke) {
	c.metrics.UnchokeCount.Inc()

	c.Lock()
	defer c.Unlock()
	c.choked = false
}

func (c *channel) HandlePing(v Ping) {
	c.metrics.PingCount.Inc()

	c.w.Write(&Pong{v.Nonce})
}

func (c *channel) HandlePong(v Pong) {
	c.metrics.PongCount.Inc()

	c.peer.Lock()
	c.peer.AddRTTSample(0, binmap.Bin(v.Nonce.Value))
	c.peer.Unlock()
}

func newChannelMetrics(id uint32) channelMetrics {
	label := strconv.FormatUint(uint64(id), 10)
	return channelMetrics{
		id:             label,
		HandshakeCount: channelMessageCount.WithLabelValues(label, "handshake"),
		DataCount:      channelMessageCount.WithLabelValues(label, "data"),
		AckCount:       channelMessageCount.WithLabelValues(label, "ack"),
		HaveCount:      channelMessageCount.WithLabelValues(label, "have"),
		RequestCount:   channelMessageCount.WithLabelValues(label, "request"),
		CancelCount:    channelMessageCount.WithLabelValues(label, "cancel"),
		ChokeCount:     channelMessageCount.WithLabelValues(label, "choke"),
		UnchokeCount:   channelMessageCount.WithLabelValues(label, "unchoke"),
		PingCount:      channelMessageCount.WithLabelValues(label, "ping"),
		PongCount:      channelMessageCount.WithLabelValues(label, "pong"),
	}
}

type channelMetrics struct {
	id             string
	HandshakeCount prometheus.Counter
	DataCount      prometheus.Counter
	AckCount       prometheus.Counter
	HaveCount      prometheus.Counter
	RequestCount   prometheus.Counter
	CancelCount    prometheus.Counter
	ChokeCount     prometheus.Counter
	UnchokeCount   prometheus.Counter
	PingCount      prometheus.Counter
	PongCount      prometheus.Counter
}

func (m *channelMetrics) Delete() {
	channelMessageCount.DeleteLabelValues(m.id, "handshake")
	channelMessageCount.DeleteLabelValues(m.id, "data")
	channelMessageCount.DeleteLabelValues(m.id, "ack")
	channelMessageCount.DeleteLabelValues(m.id, "have")
	channelMessageCount.DeleteLabelValues(m.id, "request")
	channelMessageCount.DeleteLabelValues(m.id, "cancel")
	channelMessageCount.DeleteLabelValues(m.id, "choke")
	channelMessageCount.DeleteLabelValues(m.id, "unchoke")
	channelMessageCount.DeleteLabelValues(m.id, "ping")
	channelMessageCount.DeleteLabelValues(m.id, "pong")
}

type channels []*channel

func (c *channels) Insert(v *channel) {
	*c = append(*c, v)
}

func (c *channels) Remove(v *channel) {
	old := *c
	for i, iv := range old {
		if iv == v {

			copy(old[i:], old[i+1:])
			*c = old[:len(old)-1]
		}
	}
}
