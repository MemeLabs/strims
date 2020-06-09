package vpn

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"
	"math"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/aead/ecdh"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var (
	frameReadCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_read_count",
		Help: "The total number of frames read",
	})
	frameReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_read_bytes",
		Help: "The total number of frame bytes read",
	})
	frameHandlerNotFoundCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_hander_not_found_count",
		Help: "The total number of unhandled frames",
	})
	frameHandlerErrorCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_handler_error_count",
		Help: "The total number of frame handler errors",
	})
)

func newPeer(logger *zap.Logger, link Link, hostKey *pb.Key) (*Peer, error) {
	// TODO: use io timeout?
	validDuration := time.Second * 10

	signingCert, err := dao.NewSelfSignedCertificate(
		hostKey,
		pb.KeyUsage_KEY_USAGE_SIGN|pb.KeyUsage_KEY_USAGE_PEER,
		validDuration,
	)
	if err != nil {
		panic(err)
	}

	key, pub, err := ecdh.X25519().GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	pubBytes := pub.([32]byte)
	cipherCSR := &pb.CertificateRequest{
		Key:      pubBytes[:],
		KeyType:  pb.KeyType_KEY_TYPE_X25519,
		KeyUsage: uint32(pb.KeyUsage_KEY_USAGE_ENCIPHERMENT),
	}
	cipherCert, err := dao.SignCertificateRequest(cipherCSR, validDuration, hostKey)
	if err != nil {
		return nil, err
	}
	cipherCert.ParentOneof = &pb.Certificate_Parent{Parent: signingCert}

	var iv [16]byte
	if _, err := rand.Read(iv[:]); err != nil {
		return nil, err
	}

	err = WriteProtoStream(link, &pb.PeerInit{
		ProtocolVersion: 1,
		Certificate:     cipherCert,
		Iv:              iv[:],
	})
	if err != nil {
		return nil, err
	}

	var init pb.PeerInit
	if err = ReadProtoStream(link, &init); err != nil {
		return nil, err
	}

	if err := dao.VerifyCertificate(init.Certificate); err != nil {
		return nil, err
	}
	if init.Certificate.GetParent() == nil {
		return nil, errors.New("invalid peer certificate")
	}

	if err := ecdh.X25519().Check(init.Certificate.Key); err != nil {
		return nil, err
	}
	block, err := aes.NewCipher(ecdh.X25519().ComputeSecret(key, init.Certificate.Key))
	if err != nil {
		return nil, err
	}

	_ = block
	// link = &cipherLink{
	// 	writeStream: cipher.NewCFBEncrypter(block, iv[:]),
	// 	readStream:  cipher.NewCFBDecrypter(block, init.Iv),
	// 	link:        link,
	// }

	p := &Peer{
		logger:       logger,
		Link:         link,
		Certificate:  init.Certificate.GetParent(),
		handlers:     map[uint16]FrameHandler{},
		reservations: map[uint16]struct{}{},
		done:         make(chan struct{}),
	}
	return p, nil
}

// Peer ...
type Peer struct {
	logger           *zap.Logger
	Link             Link
	Certificate      *pb.Certificate
	handlersLock     sync.Mutex
	handlers         map[uint16]FrameHandler
	reservationsLock sync.Mutex
	reservations     map[uint16]struct{}
	done             chan struct{}
	closeOnce        sync.Once
}

func (p *Peer) run() {
	for {
		var f Frame
		if _, err := f.ReadFrom(p.Link); err != nil {
			p.logger.Info("failed to read frame", zap.Error(err))
			break
		}
		frameReadCount.Inc()
		frameReadBytes.Add(float64(len(f.Body)))

		h := p.Handler(f.Header.Port)
		if h == nil {
			frameHandlerNotFoundCount.Inc()
			continue
		}

		if err := h(p, f); err != nil {
			p.logger.Warn("failed to run frame handler", zap.Error(err))
			frameHandlerErrorCount.Inc()
		}

		f.Free()
	}

	p.Close()
}

// Close ...
func (p *Peer) Close() {
	p.closeOnce.Do(func() {
		close(p.done)
		p.Link.Close()
	})
}

// Done ...
func (p *Peer) Done() <-chan struct{} {
	return p.done
}

// SetHandler ...
func (p *Peer) SetHandler(port uint16, h FrameHandler) {
	p.handlersLock.Lock()
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()
	defer p.handlersLock.Unlock()

	p.reservations[port] = struct{}{}
	p.handlers[port] = h
}

// RemoveHandler ...
func (p *Peer) RemoveHandler(port uint16) {
	p.handlersLock.Lock()
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()
	defer p.handlersLock.Unlock()

	delete(p.handlers, port)
	delete(p.reservations, port)
}

// Handler ...
func (p *Peer) Handler(port uint16) FrameHandler {
	p.handlersLock.Lock()
	defer p.handlersLock.Unlock()
	return p.handlers[port]
}

// ReservePort ...
func (p *Peer) ReservePort() (uint16, error) {
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()

	for {
		port, err := randUint16(math.MaxUint16 - reservedPortCount)
		if err != nil {
			return 0, err
		}
		port += reservedPortCount

		if _, ok := p.reservations[port]; !ok {
			p.reservations[port] = struct{}{}
			return port, nil
		}
	}
}

// ReleasePort ...
func (p *Peer) ReleasePort(port uint16) {
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()

	delete(p.reservations, port)
}

// FrameHandler ...
type FrameHandler func(p *Peer, f Frame) error

type cipherLink struct {
	readStream  cipher.Stream
	writeLock   sync.Mutex
	writeStream cipher.Stream
	link        Link
}

func (c *cipherLink) Read(p []byte) (int, error) {
	n, err := c.link.Read(p)
	if err != nil {
		return 0, err
	}
	c.readStream.XORKeyStream(p[:n], p[:n])
	return n, nil
}

func (c *cipherLink) Write(p []byte) (int, error) {
	b := frameBuffer(uint16(len(p)))
	defer freeFrameBuffer(b)

	c.writeLock.Lock()
	defer c.writeLock.Unlock()
	c.writeStream.XORKeyStream(b[:len(p)], p)

	return c.link.Write(b[:len(p)])
}

func (c *cipherLink) MTU() int {
	return c.link.MTU()
}

func (c *cipherLink) Close() error {
	return c.link.Close()
}
