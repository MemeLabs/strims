package vpn

import (
	"crypto/rand"
	"errors"
	"io"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/mpc"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

// NetworkBroker ...
type NetworkBroker interface {
	BrokerPeer(c ReadWriteFlusher) (NetworkBrokerPeer, error)
}

// NetworkBrokerPeer ...
type NetworkBrokerPeer interface {
	Init(discriminator uint16, keys [][]byte) error
	InitRequired() <-chan struct{}
	Keys() <-chan [][]byte
	Close()
}

// WithNetworkBroker ...
func WithNetworkBroker(b NetworkBroker) HostOption {
	return func(host *Host) error {
		host.networkBroker = b
		return nil
	}
}

// NewNetworkBroker ...
func NewNetworkBroker(logger *zap.Logger) NetworkBroker {
	return &networkBroker{
		logger: logger,
	}
}

// networkBroker ...
type networkBroker struct {
	logger *zap.Logger
}

// BrokerPeer ...
func (h *networkBroker) BrokerPeer(c ReadWriteFlusher) (NetworkBrokerPeer, error) {
	return newNetworkBrokerPeer(h.logger, c), nil
}

func newNetworkBrokerPeer(logger *zap.Logger, c ReadWriteFlusher) *networkBrokerPeer {
	p := &networkBrokerPeer{
		logger:       logger,
		c:            c,
		localParams:  make(chan networkBrokerLocalParams, 1),
		initRequired: make(chan struct{}, 1),
		initDone:     make(chan error, 1),
		keys:         make(chan [][]byte, 1),
	}

	go func() {
		if err := p.readInits(); err != nil {
			panic(err)
		}
	}()

	return p
}

type networkBrokerLocalParams struct {
	discriminator uint16
	keys          [][]byte
}

type networkBrokerPeer struct {
	logger       *zap.Logger
	cLock        sync.Mutex
	c            ReadWriteFlusher
	localParams  chan networkBrokerLocalParams
	initLock     sync.Mutex
	initRequired chan struct{}
	initDone     chan error
	keys         chan [][]byte
	closeOnce    sync.Once
}

func (p *networkBrokerPeer) Init(discriminator uint16, keys [][]byte) error {
	go func() {
		p.initLock.Lock()
		defer p.initLock.Unlock()

		p.logger.Debug("starting network negotiation", zap.Int("keys", len(keys)))

		p.localParams <- networkBrokerLocalParams{discriminator, keys}

		if err := p.sendInit(discriminator, keys); err != nil {
			p.logger.Error("sending negotiation init failed", zap.Error(err))
			return
		}

		if err := <-p.initDone; err != nil {
			p.logger.Error("network negotiation failed", zap.Error(err))
			return
		}

		p.logger.Debug("finished network negotiation")
	}()
	return nil
}

func (p *networkBrokerPeer) InitRequired() <-chan struct{} {
	return p.initRequired
}

func (p *networkBrokerPeer) Keys() <-chan [][]byte {
	return p.keys
}

func (p *networkBrokerPeer) Close() {
	p.closeOnce.Do(func() {
		close(p.localParams)
		close(p.initRequired)
		close(p.initDone)
		close(p.keys)
	})
}

func (p *networkBrokerPeer) readInits() (err error) {
	defer p.Close()

	for {
		var handshake pb.NetworkHandshake
		if err := ReadProtoStream(p.c, &handshake); err != nil {
			return err
		}

		switch b := handshake.Body.(type) {
		case *pb.NetworkHandshake_Init_:
			err = p.handleInit(b.Init)
		default:
			err = errors.New("unexpected network handshake type")
		}

		p.initDone <- err

		if err != nil {
			p.logger.Error("error reading broker inits", zap.Error(err))
			return err
		}
	}
}

func (p *networkBrokerPeer) sendInit(discriminator uint16, keys [][]byte) error {
	p.cLock.Lock()
	defer p.cLock.Unlock()

	err := WriteProtoStream(p.c, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_Init_{
			Init: &pb.NetworkHandshake_Init{
				KeyCount:      int32(len(keys)),
				Discriminator: uint32(discriminator),
			},
		},
	})
	if err != nil {
		return err
	}
	if err := p.c.Flush(); err != nil {
		return err
	}
	return nil
}

func (p *networkBrokerPeer) awaitLocalParams() (uint16, [][]byte, error) {
	var l networkBrokerLocalParams
	var ok bool

	select {
	case l, ok = <-p.localParams:
	default:
		p.initRequired <- struct{}{}

		select {
		case l, ok = <-p.localParams:
		case <-time.After(10 * time.Second):
			return 0, nil, errors.New("timeout")
		}
	}

	if !ok {
		return 0, nil, errors.New("broker closed")
	}
	return l.discriminator, l.keys, nil
}

func (p *networkBrokerPeer) handleInit(init *pb.NetworkHandshake_Init) error {
	discriminator, keys, err := p.awaitLocalParams()
	if err != nil {
		return err
	}

	// communication cost for the PSZ sender scales better than the receiver.
	if int(init.KeyCount) < len(keys) || (int(init.KeyCount) == len(keys) && uint16(init.Discriminator) > discriminator) {
		return p.exchangeKeysAsSender(keys)
	}

	keys, err = p.exchangeKeysAsReceiver(keys)
	if err != nil {
		return err
	}

	p.keys <- keys
	return err
}

func (p *networkBrokerPeer) exchangeKeysAsSender(keys [][]byte) error {
	p.cLock.Lock()
	defer p.cLock.Unlock()

	rng, err := newRNG()
	if err != nil {
		return err
	}
	ot, err := mpc.NewChaoOrlandiSender(p.c, rng)
	if err != nil {
		return err
	}
	ote, err := mpc.NewKOSReceiver(p.c, ot, rng)
	if err != nil {
		return err
	}
	oprf, err := mpc.NewKKRTSender(p.c, ote, rng)
	if err != nil {
		return err
	}
	psi, err := mpc.NewPSZSender(oprf)
	if err != nil {
		return err
	}
	err = psi.Send(p.c, keys, rng)
	if err != nil {
		return err
	}

	return nil
}

func (p *networkBrokerPeer) exchangeKeysAsReceiver(keys [][]byte) ([][]byte, error) {
	p.cLock.Lock()
	defer p.cLock.Unlock()

	rng, err := newRNG()
	if err != nil {
		return nil, err
	}
	ot, err := mpc.NewChaoOrlandiReceiver(p.c, rng)
	if err != nil {
		return nil, err
	}
	ote, err := mpc.NewKOSSender(p.c, ot, rng)
	if err != nil {
		return nil, err
	}
	oprf, err := mpc.NewKKRTReceiver(p.c, ote, rng)
	if err != nil {
		return nil, err
	}
	psi, err := mpc.NewPSZReceiver(oprf)
	if err != nil {
		return nil, err
	}
	results, err := psi.Receive(p.c, keys, rng)
	if err != nil {
		return nil, err
	}

	return results, nil
}

// ReadWriteFlusher ...
type ReadWriteFlusher interface {
	io.ReadWriter
	Flush() error
}

func newRNG() (*mpc.AESRNG, error) {
	var seed [16]byte
	if _, err := rand.Read(seed[:]); err != nil {
		return nil, err
	}
	return mpc.NewAESRNG(seed[:])
}
