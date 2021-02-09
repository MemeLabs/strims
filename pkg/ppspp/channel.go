package ppspp

import (
	"encoding/hex"
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var channelMessageCount = promauto.NewCounterVec(prometheus.CounterOpts{
	Name: "strims_ppspp_channel",
	Help: "The total number of ppspp messages read per channel",
}, []string{"swarm_id", "peer_id", "direction", "type"})

var (
	errNoVersionOption                    = errors.New("handshake missing VersionOption")
	errNoMinimumVersionOption             = errors.New("handshake missing MinimumVersionOption")
	errNoLiveWindowOption                 = errors.New("handshake missing LiveWindowOption")
	errNoChunkSizeOption                  = errors.New("handshake missing ChunkSizeOption")
	errNoChunksPerSignatureOption         = errors.New("handshake missing ChunksPerSignatureOption")
	errNoSwarmIdentifierOption            = errors.New("handshake missing SwarmIdentifierOption")
	errNoContentIntegrityProtectionMethod = errors.New("handshake missing ContentIntegrityProtectionMethod")
	errNoMerkleHashTreeFunction           = errors.New("handshake missing MerkleHashTreeFunction")
	errNoLiveSignatureAlgorithm           = errors.New("handshake missing LiveSignatureAlgorithm")

	errIncompatibleVersionOption                    = errors.New("incompatible VersionOption")
	errIncompatibleMinimumVersionOption             = errors.New("incompatible MinimumVersionOption")
	errIncompatibleChunkSizeOption                  = errors.New("incompatible ChunkSizeOption")
	errIncompatibleChunksPerSignatureOption         = errors.New("incompatible ChunksPerSignatureOption")
	errIncompatibleSwarmIdentifierOption            = errors.New("incompatible SwarmIdentifierOption")
	errIncompatibleContentIntegrityProtectionMethod = errors.New("incompatible ContentIntegrityProtectionMethod")
	errIncompatibleMerkleHashTreeFunction           = errors.New("incompatible MerkleHashTreeFunction")
	errIncompatibleLiveSignatureAlgorithm           = errors.New("incompatible LiveSignatureAlgorithm")
)

var nextChannelID uint64

// Channel ...
// TODO: do we still need this?
type Channel interface {
	HandleMessage(b []byte) (int, error)
}

// OpenChannel ...
func OpenChannel(logger *zap.Logger, p *Peer, s *Swarm, conn WriteFlushCloser) (*ChannelReader, error) {
	c := newChannel()
	cw := newChannelWriter(logger, newChannelWriterMetrics(s, p), c, conn)
	cr := newChannelReader(logger, newChannelReaderMetrics(s, p), c, p, s)

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

	deleteChannelWriterMetrics(s, p)
	deleteChannelReaderMetrics(s, p)
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
		Nonce: codec.Nonce{Value: uint64(c.pongNonce)},
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
	h := codec.Handshake{
		Options: []codec.ProtocolOption{
			codec.NewSwarmIdentifierProtocolOption(swarm.ID()),
			&codec.VersionProtocolOption{Value: 2},
			&codec.MinimumVersionProtocolOption{Value: 2},
			&codec.LiveWindowProtocolOption{Value: uint32(swarm.liveWindow())},
			&codec.ChunkSizeProtocolOption{Value: uint32(swarm.chunkSize())},
			&codec.ContentIntegrityProtectionMethodProtocolOption{Value: uint8(swarm.contentIntegrityProtectionMethod())},
			&codec.MerkleHashTreeFunctionProtocolOption{Value: uint8(swarm.merkleHashTreeFunction())},
			&codec.LiveSignatureAlgorithmProtocolOption{Value: uint8(swarm.liveSignatureAlgorithm())},
			&codec.ChunksPerSignatureProtocolOption{Value: uint32(swarm.chunksPerSignature())},
		},
	}

	return h
}

func newChannelWriter(logger *zap.Logger, metrics channelWriterMetrics, channel *channel, conn WriteFlushCloser) *channelWriter {
	return &channelWriter{
		channel: channel,
		w:       codec.NewWriter(conn, conn.MTU()),
		metrics: metrics,
	}
}

type channelWriter struct {
	*channel
	w       codec.Writer
	metrics channelWriterMetrics
}

func (c *channelWriter) Flush() error {
	return c.w.Flush()
}

func (c *channelWriter) Cap() int {
	return c.w.Cap()
}

func (c *channelWriter) Len() int {
	return c.w.Len()
}

func (c *channelWriter) Dirty() bool {
	return c.w.Dirty()
}

func (c *channelWriter) WriteHandshake(m codec.Handshake) (int, error) {
	c.metrics.HandshakeCount.Inc()
	return c.w.WriteHandshake(m)
}

func (c *channelWriter) WriteAck(m codec.Ack) (int, error) {
	c.metrics.AckCount.Inc()
	return c.w.WriteAck(m)
}

func (c *channelWriter) WriteHave(m codec.Have) (int, error) {
	c.metrics.HaveCount.Inc()
	return c.w.WriteHave(m)
}

func (c *channelWriter) WriteData(m codec.Data) (int, error) {
	c.metrics.DataCount.Inc()
	c.metrics.ChunkCount.Add(float64(m.Address.Bin().BaseLength()))
	return c.w.WriteData(m)
}

func (c *channelWriter) WriteIntegrity(m codec.Integrity) (int, error) {
	c.metrics.IntegrityCount.Inc()
	return c.w.WriteIntegrity(m)
}

func (c *channelWriter) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	c.metrics.SignedIntegrityCount.Inc()
	return c.w.WriteSignedIntegrity(m)
}

func (c *channelWriter) WriteRequest(m codec.Request) (int, error) {
	c.metrics.RequestCount.Inc()
	return c.w.WriteRequest(m)
}

func (c *channelWriter) WritePing(m codec.Ping) (int, error) {
	c.metrics.PingCount.Inc()
	return c.w.WritePing(m)
}

func (c *channelWriter) WritePong(m codec.Pong) (int, error) {
	c.metrics.PongCount.Inc()
	return c.w.WritePong(m)
}

func (c *channelWriter) WriteCancel(m codec.Cancel) (int, error) {
	c.metrics.CancelCount.Inc()
	return c.w.WriteCancel(m)
}

func newChannelReader(logger *zap.Logger, metrics channelReaderMetrics, channel *channel, peer *Peer, swarm *Swarm) *ChannelReader {
	return &ChannelReader{
		channel: channel,
		r: codec.Reader{
			ChunkSize:              swarm.chunkSize(),
			IntegrityHashSize:      swarm.merkleHashTreeFunction().HashSize(),
			IntegritySignatureSize: swarm.liveSignatureAlgorithm().SignatureSize(),
			Handler: &channelMessageHandler{
				logger:   logger,
				swarm:    swarm,
				peer:     peer,
				channel:  channel,
				metrics:  metrics,
				verifier: swarm.verifier.ChannelVerifier(),
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
	logger   *zap.Logger
	swarm    *Swarm
	peer     *Peer
	channel  *channel
	metrics  channelReaderMetrics
	verifier integrity.ChannelVerifier
}

func (c *channelMessageHandler) HandleHandshake(v codec.Handshake) error {
	c.metrics.HandshakeCount.Inc()

	if swarmID, ok := v.Options.Find(codec.SwarmIdentifierOption); ok {
		if !c.swarm.ID().Equals(NewSwarmID(*swarmID.(*codec.SwarmIdentifierProtocolOption))) {
			return errIncompatibleSwarmIdentifierOption
		}
	} else {
		return errNoSwarmIdentifierOption
	}

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

	if chunksPerSignature, ok := v.Options.Find(codec.ChunksPerSignatureOption); ok {
		if chunksPerSignature.(*codec.ChunksPerSignatureProtocolOption).Value != uint32(c.swarm.chunksPerSignature()) {
			return errIncompatibleChunksPerSignatureOption
		}
	} else {
		return errNoChunksPerSignatureOption
	}

	if contentIntegrityProtectionMethod, ok := v.Options.Find(codec.ContentIntegrityProtectionMethodOption); ok {
		if contentIntegrityProtectionMethod.(*codec.ContentIntegrityProtectionMethodProtocolOption).Value != uint8(c.swarm.contentIntegrityProtectionMethod()) {
			return errIncompatibleContentIntegrityProtectionMethod
		}
	} else {
		return errNoContentIntegrityProtectionMethod
	}

	if merkleHashTreeFunction, ok := v.Options.Find(codec.MerkleHashTreeFunctionOption); ok {
		if merkleHashTreeFunction.(*codec.MerkleHashTreeFunctionProtocolOption).Value != uint8(c.swarm.merkleHashTreeFunction()) {
			return errIncompatibleMerkleHashTreeFunction
		}
	} else {
		return errNoMerkleHashTreeFunction
	}

	if liveSignatureAlgorithm, ok := v.Options.Find(codec.LiveSignatureAlgorithmOption); ok {
		if liveSignatureAlgorithm.(*codec.LiveSignatureAlgorithmProtocolOption).Value != uint8(c.swarm.liveSignatureAlgorithm()) {
			return errIncompatibleLiveSignatureAlgorithm
		}
	} else {
		return errNoLiveSignatureAlgorithm
	}

	// TODO: send rightmost signed munro
	c.channel.setAddedBins(c.swarm.loadedBins())

	return nil
}

func (c *channelMessageHandler) HandleData(m codec.Data) {
	c.metrics.DataCount.Inc()
	c.metrics.ChunkCount.Add(float64(m.Address.Bin().BaseLength()))

	if v := c.verifier.ChunkVerifier(m.Address.Bin()); v != nil {
		if verified, err := v.Verify(m.Address.Bin(), m.Data); !verified {
			c.metrics.InvalidDataCount.Inc()
			c.metrics.InvalidChunkCount.Add(float64(m.Address.Bin().BaseLength()))
			c.swarm.bins.ResetRequested(m.Address.Bin())
			c.logger.Debug(
				"invalid data",
				logutil.ByteHex("peer", c.peer.id),
				zap.Stringer("swarm", c.swarm.id),
				zap.Uint64("bin", uint64(m.Address.Bin())),
				zap.Error(err),
			)
			return
		}
	}

	c.channel.enqueueAck(codec.Ack{
		Address: m.Address,
		DelaySample: codec.DelaySample{
			Duration: iotime.Load().Sub(m.Timestamp.Time),
		},
	})

	ok := c.swarm.pubSub.Publish(store.Chunk{
		Bin:  m.Address.Bin(),
		Data: m.Data,
	})

	if ok {
		c.peer.addReceivedChunk(m.Address.Bin().BaseLength())
	}
	c.peer.addRTTSample(c.channel.id, m.Address.Bin(), 0)
}

func (c *channelMessageHandler) HandleIntegrity(m codec.Integrity) {
	c.metrics.IntegrityCount.Inc()

	if v := c.verifier.ChunkVerifier(m.Address.Bin()); v != nil {
		v.SetIntegrity(m.Address.Bin(), m.Hash)
	}
}

func (c *channelMessageHandler) HandleSignedIntegrity(m codec.SignedIntegrity) {
	c.metrics.SignedIntegrityCount.Inc()

	if v := c.verifier.ChunkVerifier(m.Address.Bin()); v != nil {
		v.SetSignedIntegrity(m.Address.Bin(), m.Timestamp.Time, m.Signature)
	}
}

func (c *channelMessageHandler) HandleAck(m codec.Ack) {
	c.metrics.AckCount.Inc()

	if c.channel.addAckedBin(m.Address.Bin()) {
		// TODO: ack queuing delay
		c.peer.addRTTSample(c.channel.id, m.Address.Bin(), 0)
		c.peer.addDelaySample(m.DelaySample.Duration, c.swarm.chunkSize())
		c.swarm.bins.AddAvailable(m.Address.Bin())
	}
}

func (c *channelMessageHandler) HandleHave(v codec.Have) {
	c.metrics.HaveCount.Inc()
	c.channel.addAvailableBin(v.Bin())
	c.swarm.bins.AddAvailable(v.Bin())
}

func (c *channelMessageHandler) HandleRequest(m codec.Request) {
	c.metrics.RequestCount.Inc()
	c.channel.addRequestedBin(m.Bin())
}

func (c *channelMessageHandler) HandleCancel(m codec.Cancel) {
	c.metrics.CancelCount.Inc()
	c.channel.addCancelledBins(m.Address.Bin())
}

func (c *channelMessageHandler) HandleChoke(m codec.Choke) {
	c.metrics.ChokeCount.Inc()
	c.channel.setChoked(true)
}

func (c *channelMessageHandler) HandleUnchoke(m codec.Unchoke) {
	c.metrics.UnchokeCount.Inc()
	c.channel.setChoked(false)
}

func (c *channelMessageHandler) HandlePing(m codec.Ping) {
	c.metrics.PingCount.Inc()
	c.channel.enqueuePong(binmap.Bin(m.Nonce.Value))
}

func (c *channelMessageHandler) HandlePong(m codec.Pong) {
	c.metrics.PongCount.Inc()
	// c.peer.addRTTSample(c.channel.id, binmap.Bin(m.Nonce.Value), time.Duration(m.Delay))
	c.peer.addRTTSample(c.channel.id, binmap.Bin(m.Nonce.Value), 0)
}

func newChannelReaderMetrics(s *Swarm, p *Peer) channelReaderMetrics {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()
	direction := "in"

	return channelReaderMetrics{
		channelMetrics:    newChannelMetrics(s, p, direction),
		InvalidDataCount:  channelMessageCount.WithLabelValues(swarmID, peerID, direction, "invalid_data"),
		InvalidChunkCount: channelMessageCount.WithLabelValues(swarmID, peerID, direction, "invalid_chunk"),
	}
}

type channelReaderMetrics struct {
	channelMetrics
	InvalidDataCount  prometheus.Counter
	InvalidChunkCount prometheus.Counter
}

func deleteChannelReaderMetrics(s *Swarm, p *Peer) {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()
	direction := "in"

	deleteChannelMetrics(s, p, direction)
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "invalid_data")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "invalid_chunk")
}

func newChannelWriterMetrics(s *Swarm, p *Peer) channelWriterMetrics {
	return channelWriterMetrics{
		channelMetrics: newChannelMetrics(s, p, "out"),
	}
}

type channelWriterMetrics struct {
	channelMetrics
}

func deleteChannelWriterMetrics(s *Swarm, p *Peer) {
	deleteChannelMetrics(s, p, "out")
}

func newChannelMetrics(s *Swarm, p *Peer, direction string) channelMetrics {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()

	return channelMetrics{
		HandshakeCount:       channelMessageCount.WithLabelValues(swarmID, peerID, direction, "handshake_message"),
		DataCount:            channelMessageCount.WithLabelValues(swarmID, peerID, direction, "data_message"),
		ChunkCount:           channelMessageCount.WithLabelValues(swarmID, peerID, direction, "chunk"),
		IntegrityCount:       channelMessageCount.WithLabelValues(swarmID, peerID, direction, "integrity_message"),
		SignedIntegrityCount: channelMessageCount.WithLabelValues(swarmID, peerID, direction, "signed_integrity_message"),
		AckCount:             channelMessageCount.WithLabelValues(swarmID, peerID, direction, "ack_message"),
		HaveCount:            channelMessageCount.WithLabelValues(swarmID, peerID, direction, "have_message"),
		RequestCount:         channelMessageCount.WithLabelValues(swarmID, peerID, direction, "request_message"),
		CancelCount:          channelMessageCount.WithLabelValues(swarmID, peerID, direction, "cancel_message"),
		ChokeCount:           channelMessageCount.WithLabelValues(swarmID, peerID, direction, "choke_message"),
		UnchokeCount:         channelMessageCount.WithLabelValues(swarmID, peerID, direction, "unchoke_message"),
		PingCount:            channelMessageCount.WithLabelValues(swarmID, peerID, direction, "ping_message"),
		PongCount:            channelMessageCount.WithLabelValues(swarmID, peerID, direction, "pong_message"),
	}
}

type channelMetrics struct {
	HandshakeCount       prometheus.Counter
	DataCount            prometheus.Counter
	ChunkCount           prometheus.Counter
	IntegrityCount       prometheus.Counter
	SignedIntegrityCount prometheus.Counter
	AckCount             prometheus.Counter
	HaveCount            prometheus.Counter
	RequestCount         prometheus.Counter
	CancelCount          prometheus.Counter
	ChokeCount           prometheus.Counter
	UnchokeCount         prometheus.Counter
	PingCount            prometheus.Counter
	PongCount            prometheus.Counter
}

func deleteChannelMetrics(s *Swarm, p *Peer, direction string) {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()

	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "handshake_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "data_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "chunk")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "integrity_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "signed_integrity_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "ack_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "have_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "request_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "cancel_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "choke_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "unchoke_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "ping_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "pong_message")
}
