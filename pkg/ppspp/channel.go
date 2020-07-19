package ppspp

import (
	"errors"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var channelMessageCount = promauto.NewCounterVec(prometheus.CounterOpts{
	Name: "strims_ppspp_message_count",
	Help: "The total number of ppspp messages read per channel",
}, []string{"channel_id", "direction", "type"})

var (
	errNoVersionOption         = errors.New("handshake missing VersionOption")
	errNoMinimumVersionOption  = errors.New("handshake missing MinimumVersionOption")
	errNoLiveWindowOption      = errors.New("handshake missing LiveWindowOption")
	errNoChunkSizeOption       = errors.New("handshake missing ChunkSizeOption")
	errNoSwarmIdentifierOption = errors.New("handshake missing SwarmIdentifierOption")

	errIncompatibleVersionOption         = errors.New("incompatible VersionOption")
	errIncompatibleMinimumVersionOption  = errors.New("incompatible MinimumVersionOption")
	errIncompatibleChunkSizeOption       = errors.New("incompatible ChunkSizeOption")
	errIncompatibleSwarmIdentifierOption = errors.New("incompatible SwarmIdentifierOption")
)

var nextChannelID uint64

// Channel ...
// TODO: do we still need this?
type Channel interface {
	HandleMessage(b []byte) (int, error)
}

// OpenChannel ...
func OpenChannel(p *Peer, s *Swarm, conn WriteFlushCloser) (*ChannelReader, error) {
	c := newChannel()
	cw := newChannelWriter(c, conn)
	cr := newChannelReader(c, p, s)

	if _, err := cw.WriteHandshake(newHandshake(s)); err != nil {
		return nil, err
	}
	if err := cw.Flush(); err != nil {
		return nil, err
	}

	p.addChannel(s, cw)
	s.addChannel(p, c)

	return cr, nil
}

// CloseChannel ...
func CloseChannel(p *Peer, s *Swarm) {
	p.removeChannel(s)
	s.removeChannel(p)
}

// newchannel ...
func newChannel() *channel {
	return &channel{
		id:                  atomic.AddUint64(&nextChannelID, 1),
		addedBins:           binmap.New(),
		requestedBins:       binmap.New(),
		availableBins:       binmap.New(),
		unackedBins:         binmap.New(),
		sentBinHistory:      newBinTimeoutQueue(32),
		requestedBinHistory: newBinTimeoutQueue(32),
		close:               make(chan struct{}),
	}
}

// channel ...
type channel struct {
	sync.Mutex
	id                  uint64
	liveWindow          int
	choked              bool
	addedBins           *binmap.Map      // bins to send HAVEs for
	requestedBins       *binmap.Map      // bins to send DATA for
	availableBins       *binmap.Map      // bins the peer claims to have
	unackedBins         *binmap.Map      // sent bins that have not been acked
	sentBinHistory      *binTimeoutQueue // recently sent bins
	requestedBinHistory *binTimeoutQueue // bins recently requested from the peer
	acks                []codec.Ack
	pongNonce           binmap.Bin
	pongTime            time.Time
	closeOnce           sync.Once
	close               chan struct{}
}

func (c *channel) setLiveWindow(v int) {
	c.Lock()
	c.liveWindow = v
	c.Unlock()
}

func (c *channel) setChoked(v bool) {
	c.Lock()
	c.choked = true
	c.Unlock()
}

func (c *channel) setAddedBins(m *binmap.Map) {
	c.Lock()
	c.addedBins = m
	c.Unlock()
}

func (c *channel) addRequestedBin(b binmap.Bin) {
	c.Lock()
	c.requestedBins.Set(b)
	c.Unlock()
}

func (c *channel) addAvailableBin(b binmap.Bin) {
	c.Lock()
	c.availableBins.Set(b)
	c.Unlock()
}

func (c *channel) addAckedBin(b binmap.Bin) bool {
	c.Lock()
	defer c.Unlock()

	// if c.unackedBins.EmptyAt(b) {
	// 	return false
	// }

	// c.availableBins.Set(b)
	// c.unackedBins.Reset(b)

	return true
}

func (c *channel) enqueueAck(a codec.Ack) {
	// c.Lock()
	// c.acks = append(c.acks, a)
	// c.Unlock()
}

// TODO: count filled bins in b
func (c *channel) addCancelledBins(b binmap.Bin) {
	c.Lock()
	c.requestedBins.Reset(b)
	c.Unlock()
}

func (c *channel) enqueuePong(nonce binmap.Bin) {
	c.Lock()
	defer c.Unlock()

	c.pongNonce = nonce
	c.pongTime = iotime.Load()
}

func (c *channel) dequeuePong() *codec.Pong {
	c.Lock()
	defer c.Unlock()

	if c.pongNonce.IsNone() {
		return nil
	}

	p := &codec.Pong{
		Nonce: uint64(c.pongNonce),
		Delay: uint64(time.Since(c.pongTime)),
	}

	c.pongNonce = binmap.None

	return p
}

func (c *channel) Consume(sc store.Chunk) bool {
	c.Lock()
	defer c.Unlock()

	c.addedBins.Set(sc.Bin)
	return true
}

func (c *channel) Close() {
	c.closeOnce.Do(func() {
		close(c.close)
	})
}

func (c *channel) Done() <-chan struct{} {
	return c.close
}

func newHandshake(swarm *Swarm) codec.Handshake {
	return codec.Handshake{
		Options: []codec.ProtocolOption{
			&codec.VersionProtocolOption{Value: 2},
			&codec.MinimumVersionProtocolOption{Value: 2},
			&codec.LiveWindowProtocolOption{Value: uint32(swarm.liveWindow())},
			&codec.ChunkSizeProtocolOption{Value: uint32(swarm.chunkSize())},
			codec.NewSwarmIdentifierProtocolOption(swarm.ID()),
		},
	}
}

func newChannelWriter(channel *channel, conn WriteFlushCloser) *channelWriter {
	return &channelWriter{
		channel: channel,
		w:       codec.NewWriter(conn, conn.MTU()),
		metrics: newChannelMetrics(channel, "out"),
	}
}

type channelWriter struct {
	*channel
	w       codec.Writer
	metrics channelMetrics
	dirty   bool
}

func (c *channelWriter) Flush() error {
	c.dirty = false
	return c.w.Flush()
}

func (c *channelWriter) Dirty() bool {
	return c.dirty
}

func (c *channelWriter) WriteHandshake(m codec.Handshake) (int, error) {
	c.metrics.HandshakeCount.Inc()
	c.dirty = true
	return c.w.WriteHandshake(m)
}

func (c *channelWriter) WriteAck(m codec.Ack) (int, error) {
	c.metrics.AckCount.Inc()
	c.dirty = true
	return c.w.WriteAck(m)
}

func (c *channelWriter) WriteHave(m codec.Have) (int, error) {
	c.metrics.HaveCount.Inc()
	c.dirty = true
	return c.w.WriteHave(m)
}

func (c *channelWriter) WriteData(m codec.Data) (int, error) {
	c.metrics.DataCount.Inc()
	c.dirty = true
	return c.w.WriteData(m)
}

func (c *channelWriter) WriteRequest(m codec.Request) (int, error) {
	c.metrics.RequestCount.Inc()
	c.dirty = true
	return c.w.WriteRequest(m)
}

func (c *channelWriter) WritePing(m codec.Ping) (int, error) {
	c.metrics.PingCount.Inc()
	c.dirty = true
	return c.w.WritePing(m)
}

func (c *channelWriter) WritePong(m codec.Pong) (int, error) {
	c.metrics.PongCount.Inc()
	c.dirty = true
	return c.w.WritePong(m)
}

func (c *channelWriter) WriteCancel(m codec.Cancel) (int, error) {
	c.metrics.CancelCount.Inc()
	c.dirty = true
	return c.w.WriteCancel(m)
}

func newChannelReader(channel *channel, peer *Peer, swarm *Swarm) *ChannelReader {
	return &ChannelReader{
		channel: channel,
		r: codec.Reader{
			ChunkSize: swarm.chunkSize(),
			Handler: &channelMessageHandler{
				swarm:   swarm,
				peer:    peer,
				channel: channel,
				metrics: newChannelMetrics(channel, "in"),
			},
		},
	}
}

// ChannelReader ...
type ChannelReader struct {
	channel *channel
	r       codec.Reader
}

// HandleMessage ...
func (c *ChannelReader) HandleMessage(b []byte) (int, error) {
	n, err := c.r.Read(b)
	if err != nil {
		return 0, err
	}

	return n, nil
}

// Close ...
func (c *ChannelReader) Close() {
	c.channel.Close()
}

// Done ...
func (c *ChannelReader) Done() <-chan struct{} {
	return c.channel.Done()
}

type channelMessageHandler struct {
	swarm *Swarm
	// * ID()
	// * chunkSize()
	// * loadedBins()
	// * pubSub
	peer    *Peer
	channel *channel
	metrics channelMetrics
}

func (c *channelMessageHandler) HandleHandshake(v codec.Handshake) error {
	c.metrics.HandshakeCount.Inc()

	if version, ok := v.Options.Find(codec.VersionOption); ok {
		if version.(*codec.VersionProtocolOption).Value < MinimumProtocolVersion {
			return errIncompatibleVersionOption
		}
	} else {
		return errNoVersionOption
	}

	if minimumVersion, ok := v.Options.Find(codec.MinimumVersionOption); ok {
		if minimumVersion.(*codec.MinimumVersionProtocolOption).Value > ProtocolVersion {
			return errIncompatibleMinimumVersionOption
		}
	} else {
		return errNoMinimumVersionOption
	}

	if liveWindow, ok := v.Options.Find(codec.LiveWindowOption); ok {
		c.channel.setLiveWindow(int(liveWindow.(*codec.LiveWindowProtocolOption).Value))
	} else {
		return errNoLiveWindowOption
	}

	if chunkSize, ok := v.Options.Find(codec.ChunkSizeOption); ok {
		if chunkSize.(*codec.ChunkSizeProtocolOption).Value != uint32(c.swarm.chunkSize()) {
			return errIncompatibleChunkSizeOption
		}
	} else {
		return errNoChunkSizeOption
	}

	if swarmID, ok := v.Options.Find(codec.SwarmIdentifierOption); ok {
		if !c.swarm.ID().Equals(NewSwarmID(*swarmID.(*codec.SwarmIdentifierProtocolOption))) {
			return errIncompatibleSwarmIdentifierOption
		}
	} else {
		return errNoSwarmIdentifierOption
	}

	c.channel.setAddedBins(c.swarm.loadedBins())

	return nil
}

func (c *channelMessageHandler) HandleData(v codec.Data) {
	c.metrics.DataCount.Inc()

	c.channel.enqueueAck(codec.Ack{
		Address: v.Address,
		DelaySample: codec.DelaySample{
			Duration: iotime.Load().Sub(v.Timestamp.Time),
		},
	})

	ok := c.swarm.pubSub.Publish(store.Chunk{
		Bin:  v.Address.Bin(),
		Data: v.Data,
	})

	if ok {
		c.peer.addReceivedChunk()
	}
	c.peer.addRTTSample(c.channel.id, v.Address.Bin(), 0)
}

func (c *channelMessageHandler) HandleAck(v codec.Ack) {
	c.metrics.AckCount.Inc()

	if c.channel.addAckedBin(v.Address.Bin()) {
		// TODO: ack queuing delay
		c.peer.addRTTSample(c.channel.id, v.Address.Bin(), 0)
		c.peer.addDelaySample(v.DelaySample.Duration, c.swarm.chunkSize())
		c.swarm.bins.AddAvailable(v.Address.Bin())
	}
}

func (c *channelMessageHandler) HandleHave(v codec.Have) {
	c.metrics.HaveCount.Inc()
	c.channel.addAvailableBin(v.Bin())
	c.swarm.bins.AddAvailable(v.Bin())
}

func (c *channelMessageHandler) HandleRequest(v codec.Request) {
	c.metrics.RequestCount.Inc()
	c.channel.addRequestedBin(v.Bin())
}

func (c *channelMessageHandler) HandleCancel(v codec.Cancel) {
	c.metrics.CancelCount.Inc()
	c.channel.addCancelledBins(v.Address.Bin())
}

func (c *channelMessageHandler) HandleChoke(v codec.Choke) {
	c.metrics.ChokeCount.Inc()
	c.channel.setChoked(true)
}

func (c *channelMessageHandler) HandleUnchoke(v codec.Unchoke) {
	c.metrics.UnchokeCount.Inc()
	c.channel.setChoked(false)
}

func (c *channelMessageHandler) HandlePing(v codec.Ping) {
	c.metrics.PingCount.Inc()
	c.channel.enqueuePong(binmap.Bin(v.Nonce.Value))
}

func (c *channelMessageHandler) HandlePong(v codec.Pong) {
	c.metrics.PongCount.Inc()
	// c.peer.addRTTSample(c.channel.id, binmap.Bin(v.Nonce), time.Duration(v.Delay))
	c.peer.addRTTSample(c.channel.id, binmap.Bin(v.Nonce), 0)
}

func newChannelMetrics(channel *channel, direction string) channelMetrics {
	id := strconv.FormatUint(channel.id, 10)
	return channelMetrics{
		id:             id,
		direction:      direction,
		HandshakeCount: channelMessageCount.WithLabelValues(id, direction, "handshake"),
		DataCount:      channelMessageCount.WithLabelValues(id, direction, "data"),
		AckCount:       channelMessageCount.WithLabelValues(id, direction, "ack"),
		HaveCount:      channelMessageCount.WithLabelValues(id, direction, "have"),
		RequestCount:   channelMessageCount.WithLabelValues(id, direction, "request"),
		CancelCount:    channelMessageCount.WithLabelValues(id, direction, "cancel"),
		ChokeCount:     channelMessageCount.WithLabelValues(id, direction, "choke"),
		UnchokeCount:   channelMessageCount.WithLabelValues(id, direction, "unchoke"),
		PingCount:      channelMessageCount.WithLabelValues(id, direction, "ping"),
		PongCount:      channelMessageCount.WithLabelValues(id, direction, "pong"),
	}
}

type channelMetrics struct {
	id             string
	direction      string
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
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "handshake")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "data")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "ack")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "have")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "request")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "cancel")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "choke")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "unchoke")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "ping")
	channelMessageCount.DeleteLabelValues(m.id, m.direction, "pong")
}
