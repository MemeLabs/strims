package ppspp

import (
	"encoding/hex"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/errutil"
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
	errNoStreamCountOption                = errors.New("handshake missing StreamCountOption")
	errNoSwarmIdentifierOption            = errors.New("handshake missing SwarmIdentifierOption")
	errNoContentIntegrityProtectionMethod = errors.New("handshake missing ContentIntegrityProtectionMethod")
	errNoMerkleHashTreeFunction           = errors.New("handshake missing MerkleHashTreeFunction")
	errNoLiveSignatureAlgorithm           = errors.New("handshake missing LiveSignatureAlgorithm")

	errIncompatibleVersionOption                    = errors.New("incompatible VersionOption")
	errIncompatibleMinimumVersionOption             = errors.New("incompatible MinimumVersionOption")
	errIncompatibleChunkSizeOption                  = errors.New("incompatible ChunkSizeOption")
	errIncompatibleChunksPerSignatureOption         = errors.New("incompatible ChunksPerSignatureOption")
	errIncompatibleStreamCountOption                = errors.New("incompatible StreamCountOption")
	errIncompatibleSwarmIdentifierOption            = errors.New("incompatible SwarmIdentifierOption")
	errIncompatibleContentIntegrityProtectionMethod = errors.New("incompatible ContentIntegrityProtectionMethod")
	errIncompatibleMerkleHashTreeFunction           = errors.New("incompatible MerkleHashTreeFunction")
	errIncompatibleLiveSignatureAlgorithm           = errors.New("incompatible LiveSignatureAlgorithm")

	errWriterTooSmall = errors.New("new size cannot be smaller than channel header")
)

func newHandshake(swarm *Swarm) codec.Handshake {
	return codec.Handshake{
		Options: []codec.ProtocolOption{
			codec.NewSwarmIdentifierProtocolOption(swarm.ID()),
			&codec.VersionProtocolOption{Value: 2},
			&codec.MinimumVersionProtocolOption{Value: 2},
			&codec.LiveWindowProtocolOption{Value: uint32(swarm.options.LiveWindow)},
			&codec.ChunkSizeProtocolOption{Value: uint32(swarm.options.ChunkSize)},
			&codec.ContentIntegrityProtectionMethodProtocolOption{Value: uint8(swarm.options.Integrity.ProtectionMethod)},
			&codec.MerkleHashTreeFunctionProtocolOption{Value: uint8(swarm.options.Integrity.MerkleHashTreeFunction)},
			&codec.LiveSignatureAlgorithmProtocolOption{Value: uint8(swarm.options.Integrity.LiveSignatureAlgorithm)},
			&codec.ChunksPerSignatureProtocolOption{Value: uint32(swarm.options.ChunksPerSignature)},
			&codec.StreamCountProtocolOption{Value: uint16(swarm.options.StreamCount)},
		},
	}
}

func newChannelWriter(metrics channelWriterMetrics, w Conn, channel codec.Channel) *channelWriter {
	head := codec.ChannelHeader{Channel: channel}
	return &channelWriter{
		w:       w,
		cw:      codec.NewWriter(w, w.MTU()-head.ByteLen()),
		metrics: metrics,
		head:    head,
		buf:     make([]byte, head.ByteLen()),
	}
}

type channelWriter struct {
	w       Conn
	cw      codec.Writer
	metrics channelWriterMetrics
	head    codec.ChannelHeader
	buf     []byte
}

func (c *channelWriter) Len() int {
	if !c.cw.Dirty() {
		return 0
	}
	return len(c.buf) + c.cw.Len()
}

func (c *channelWriter) Resize(n int) error {
	if n < len(c.buf) {
		return errWriterTooSmall
	}
	return c.cw.Resize(n - len(c.buf))
}

func (c *channelWriter) Reset() {
	c.cw.Reset()
}

func (c *channelWriter) Flush() error {
	if !c.cw.Dirty() {
		return nil
	}

	c.head.Length = uint16(c.cw.Len())
	c.head.Marshal(c.buf)
	if _, err := c.w.Write(c.buf); err != nil {
		return err
	}

	if err := c.cw.Flush(); err != nil {
		return err
	}

	return nil
}

func (c *channelWriter) WriteHandshake(m codec.Handshake) (int, error) {
	c.metrics.HandshakeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteHandshake(m)
}

func (c *channelWriter) WriteAck(m codec.Ack) (int, error) {
	c.metrics.AckCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteAck(m)
}

func (c *channelWriter) WriteHave(m codec.Have) (int, error) {
	c.metrics.HaveCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteHave(m)
}

func (c *channelWriter) WriteData(m codec.Data) (int, error) {
	c.metrics.DataCount.Inc()
	c.metrics.ChunkCount.Add(float64(m.Address.Bin().BaseLength()))
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen() - m.Data.ByteLen()))
	c.metrics.DataBytesCount.Add(float64(m.Data.ByteLen()))
	return c.cw.WriteData(m)
}

func (c *channelWriter) WriteIntegrity(m codec.Integrity) (int, error) {
	c.metrics.IntegrityCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteIntegrity(m)
}

func (c *channelWriter) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	c.metrics.SignedIntegrityCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteSignedIntegrity(m)
}

func (c *channelWriter) WriteRequest(m codec.Request) (int, error) {
	c.metrics.RequestCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteRequest(m)
}

func (c *channelWriter) WritePing(m codec.Ping) (int, error) {
	c.metrics.PingCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WritePing(m)
}

func (c *channelWriter) WritePong(m codec.Pong) (int, error) {
	c.metrics.PongCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WritePong(m)
}

func (c *channelWriter) WriteCancel(m codec.Cancel) (int, error) {
	c.metrics.CancelCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteCancel(m)
}

func (c *channelWriter) WriteChoke(m codec.Choke) (int, error) {
	c.metrics.ChokeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteChoke(m)
}

func (c *channelWriter) WriteUnchoke(m codec.Unchoke) (int, error) {
	c.metrics.UnchokeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteUnchoke(m)
}

func (c *channelWriter) WriteStreamRequest(m codec.StreamRequest) (int, error) {
	c.metrics.StreamRequestCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteStreamRequest(m)
}

func (c *channelWriter) WriteStreamCancel(m codec.StreamCancel) (int, error) {
	c.metrics.StreamCancelCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteStreamCancel(m)
}

func (c *channelWriter) WriteStreamOpen(m codec.StreamOpen) (int, error) {
	c.metrics.StreamOpenCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteStreamOpen(m)
}

func (c *channelWriter) WriteStreamClose(m codec.StreamClose) (int, error) {
	c.metrics.StreamCloseCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.cw.WriteStreamClose(m)
}

func newChannelReader(logger *zap.Logger) *ChannelReader {
	return &ChannelReader{
		logger:   logger,
		touched:  make([]*channelReaderChannel, 0, 16),
		channels: map[codec.Channel]*channelReaderChannel{},
	}
}

// ChannelReader ...
type ChannelReader struct {
	logger   *zap.Logger
	lock     sync.Mutex
	v        uint64
	touched  []*channelReaderChannel
	channels map[codec.Channel]*channelReaderChannel
}

type channelReaderChannel struct {
	v         uint64
	scheduler ChannelScheduler
	r         codec.Reader
}

func (c *ChannelReader) openChannel(channel codec.Channel, metrics channelReaderMetrics, scheduler ChannelScheduler, swarm *Swarm) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.channels[channel] = &channelReaderChannel{
		scheduler: scheduler,
		r: codec.Reader{
			ChunkSize:              swarm.options.ChunkSize,
			IntegrityHashSize:      swarm.options.Integrity.MerkleHashTreeFunction.HashSize(),
			IntegritySignatureSize: swarm.options.Integrity.LiveSignatureAlgorithm.SignatureSize(),
			Handler: &channelMessageHandler{
				logger:    c.logger.With(zap.Stringer("swarm", swarm.id)),
				swarm:     swarm,
				scheduler: scheduler,
				metrics:   metrics,
				verifier:  swarm.verifier.ChannelVerifier(),
			},
		},
	}
}

func (c *ChannelReader) closeChannel(channel codec.Channel) {
	c.lock.Lock()
	defer c.lock.Unlock()

	delete(c.channels, channel)
}

// HandleMessage ...
func (c *ChannelReader) HandleMessage(b []byte) (err error) {
	defer func() {
		if e := recover(); e != nil {
			err = errutil.RecoverError(e)
		}
	}()

	c.lock.Lock()
	defer c.lock.Unlock()

	c.touched = c.touched[:0]
	c.v++

	for len(b) != 0 {
		var h codec.ChannelHeader
		n, err := h.Unmarshal(b)
		if err != nil {
			return err
		}
		b = b[n:]

		if rc, ok := c.channels[h.Channel]; ok {
			if rc.v != c.v {
				c.touched = append(c.touched, rc)
				rc.v = c.v
			}

			if _, err := rc.r.Read(b[:h.Length]); err != nil {
				return err
			}
		}
		b = b[h.Length:]
	}

	for _, rc := range c.touched {
		if err := rc.scheduler.HandleMessageEnd(); err != nil {
			return err
		}
	}

	return nil
}

type channelMessageHandler struct {
	logger    *zap.Logger
	swarm     *Swarm
	scheduler ChannelScheduler
	metrics   channelReaderMetrics
	verifier  integrity.ChannelVerifier
}

func (c *channelMessageHandler) HandleHandshake(v codec.Handshake) error {
	c.metrics.HandshakeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(v.ByteLen()))

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

	if _, ok := v.Options.Find(codec.LiveWindowOption); !ok {
		return errNoLiveWindowOption
	}

	if chunkSize, ok := v.Options.Find(codec.ChunkSizeOption); ok {
		if chunkSize.(*codec.ChunkSizeProtocolOption).Value != uint32(c.swarm.options.ChunkSize) {
			return errIncompatibleChunkSizeOption
		}
	} else {
		return errNoChunkSizeOption
	}

	if chunksPerSignature, ok := v.Options.Find(codec.ChunksPerSignatureOption); ok {
		if chunksPerSignature.(*codec.ChunksPerSignatureProtocolOption).Value != uint32(c.swarm.options.ChunksPerSignature) {
			return errIncompatibleChunksPerSignatureOption
		}
	} else {
		return errNoChunksPerSignatureOption
	}

	if chunksPerSignature, ok := v.Options.Find(codec.StreamCountOption); ok {
		if chunksPerSignature.(*codec.StreamCountProtocolOption).Value != uint16(c.swarm.options.StreamCount) {
			return errIncompatibleStreamCountOption
		}
	} else {
		return errNoStreamCountOption
	}

	if contentIntegrityProtectionMethod, ok := v.Options.Find(codec.ContentIntegrityProtectionMethodOption); ok {
		if contentIntegrityProtectionMethod.(*codec.ContentIntegrityProtectionMethodProtocolOption).Value != uint8(c.swarm.options.Integrity.ProtectionMethod) {
			return errIncompatibleContentIntegrityProtectionMethod
		}
	} else {
		return errNoContentIntegrityProtectionMethod
	}

	if merkleHashTreeFunction, ok := v.Options.Find(codec.MerkleHashTreeFunctionOption); ok {
		if merkleHashTreeFunction.(*codec.MerkleHashTreeFunctionProtocolOption).Value != uint8(c.swarm.options.Integrity.MerkleHashTreeFunction) {
			return errIncompatibleMerkleHashTreeFunction
		}
	} else {
		return errNoMerkleHashTreeFunction
	}

	if liveSignatureAlgorithm, ok := v.Options.Find(codec.LiveSignatureAlgorithmOption); ok {
		if liveSignatureAlgorithm.(*codec.LiveSignatureAlgorithmProtocolOption).Value != uint8(c.swarm.options.Integrity.LiveSignatureAlgorithm) {
			return errIncompatibleLiveSignatureAlgorithm
		}
	} else {
		return errNoLiveSignatureAlgorithm
	}

	liveWindow := v.Options.MustFind(codec.LiveWindowOption).(*codec.LiveWindowProtocolOption).Value
	return c.scheduler.HandleHandshake(liveWindow)
}

func (c *channelMessageHandler) HandleData(m codec.Data) error {
	c.metrics.DataCount.Inc()
	c.metrics.ChunkCount.Add(float64(m.Address.Bin().BaseLength()))
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen() - m.Data.ByteLen()))
	c.metrics.DataBytesCount.Add(float64(m.Data.ByteLen()))

	verified, err := c.verifier.ChunkVerifier(m.Address.Bin()).Verify(m.Address.Bin(), m.Data)
	if !verified {
		c.metrics.InvalidDataCount.Inc()
		c.metrics.InvalidChunkCount.Add(float64(m.Address.Bin().BaseLength()))
		c.metrics.InvalidBytesCount.Add(float64(m.Data.ByteLen()))

		c.logger.Debug(
			"invalid data",
			zap.Uint64("bin", uint64(m.Address.Bin())),
			zap.Error(err),
		)

		return c.scheduler.HandleData(m.Address.Bin(), m.Timestamp.Time, false)
	}

	c.swarm.pubSub.Publish(store.Chunk{
		Bin:  m.Address.Bin(),
		Data: m.Data,
	})
	return c.scheduler.HandleData(m.Address.Bin(), m.Timestamp.Time, true)
}

func (c *channelMessageHandler) HandleIntegrity(m codec.Integrity) error {
	c.metrics.IntegrityCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))

	if v := c.verifier.ChunkVerifier(m.Address.Bin()); v != nil {
		v.SetIntegrity(m.Address.Bin(), m.Hash)
	}
	return nil
}

func (c *channelMessageHandler) HandleSignedIntegrity(m codec.SignedIntegrity) error {
	c.metrics.SignedIntegrityCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))

	if v := c.verifier.ChunkVerifier(m.Address.Bin()); v != nil {
		v.SetSignedIntegrity(m.Address.Bin(), m.Timestamp.Time, m.Signature)
	}
	return nil
}

func (c *channelMessageHandler) HandleAck(m codec.Ack) error {
	c.metrics.AckCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleAck(m.Address.Bin(), m.DelaySample.Duration)
}

func (c *channelMessageHandler) HandleHave(m codec.Have) error {
	c.metrics.HaveCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleHave(m.Bin())
}

func (c *channelMessageHandler) HandleRequest(m codec.Request) error {
	c.metrics.RequestCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleRequest(m.Address.Bin(), m.Timestamp.Time)
}

func (c *channelMessageHandler) HandleCancel(m codec.Cancel) error {
	c.metrics.CancelCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleCancel(m.Bin())
}

func (c *channelMessageHandler) HandleChoke(m codec.Choke) error {
	c.metrics.ChokeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleChoke()
}

func (c *channelMessageHandler) HandleUnchoke(m codec.Unchoke) error {
	c.metrics.UnchokeCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleUnchoke()
}

func (c *channelMessageHandler) HandlePing(m codec.Ping) error {
	c.metrics.PingCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandlePing(m.Value)
}

func (c *channelMessageHandler) HandlePong(m codec.Pong) error {
	c.metrics.PongCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandlePong(m.Nonce.Value)
}

func (c *channelMessageHandler) HandleStreamRequest(m codec.StreamRequest) error {
	c.metrics.StreamRequestCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleStreamRequest(m.Stream, m.Address.Bin())
}

func (c *channelMessageHandler) HandleStreamCancel(m codec.StreamCancel) error {
	c.metrics.StreamCancelCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleStreamCancel(m.Stream)
}

func (c *channelMessageHandler) HandleStreamOpen(m codec.StreamOpen) error {
	c.metrics.StreamOpenCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleStreamOpen(m.Stream, m.Address.Bin())
}

func (c *channelMessageHandler) HandleStreamClose(m codec.StreamClose) error {
	c.metrics.StreamCloseCount.Inc()
	c.metrics.OverheadBytesCount.Add(float64(m.ByteLen()))
	return c.scheduler.HandleStreamClose(m.Stream)
}

func newChannelReaderMetrics(s *Swarm, p *Peer) channelReaderMetrics {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()
	direction := "in"

	return channelReaderMetrics{
		channelMetrics:    newChannelMetrics(s, p, direction),
		InvalidDataCount:  channelMessageCount.WithLabelValues(swarmID, peerID, direction, "invalid_data"),
		InvalidChunkCount: channelMessageCount.WithLabelValues(swarmID, peerID, direction, "invalid_chunk"),
		InvalidBytesCount: channelMessageCount.WithLabelValues(swarmID, peerID, direction, "invalid_bytes"),
	}
}

type channelReaderMetrics struct {
	channelMetrics
	InvalidDataCount  prometheus.Counter
	InvalidChunkCount prometheus.Counter
	InvalidBytesCount prometheus.Counter
}

func deleteChannelReaderMetrics(s *Swarm, p *Peer) {
	peerID := hex.EncodeToString(p.id)
	swarmID := s.id.String()
	direction := "in"

	deleteChannelMetrics(s, p, direction)
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "invalid_data")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "invalid_chunk")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "invalid_bytes")
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
		StreamRequestCount:   channelMessageCount.WithLabelValues(swarmID, peerID, direction, "stream_request_message"),
		StreamCancelCount:    channelMessageCount.WithLabelValues(swarmID, peerID, direction, "stream_cancel_message"),
		StreamOpenCount:      channelMessageCount.WithLabelValues(swarmID, peerID, direction, "stream_open_message"),
		StreamCloseCount:     channelMessageCount.WithLabelValues(swarmID, peerID, direction, "stream_close_message"),
		DataBytesCount:       channelMessageCount.WithLabelValues(swarmID, peerID, direction, "data_bytes"),
		OverheadBytesCount:   channelMessageCount.WithLabelValues(swarmID, peerID, direction, "overhead_bytes"),
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
	StreamRequestCount   prometheus.Counter
	StreamCancelCount    prometheus.Counter
	StreamOpenCount      prometheus.Counter
	StreamCloseCount     prometheus.Counter
	DataBytesCount       prometheus.Counter
	OverheadBytesCount   prometheus.Counter
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
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "stream_request_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "stream_cancel_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "stream_open_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "stream_close_message")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "data_bytes")
	channelMessageCount.DeleteLabelValues(swarmID, peerID, direction, "overhead_bytes")
}
