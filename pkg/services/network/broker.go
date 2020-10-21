package network

import (
	"crypto/rand"
	"errors"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/mpc"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"go.uber.org/zap"
)

// NewBrokerFactory constructs a new local broker factory
func NewBrokerFactory(logger *zap.Logger) BrokerFactory {
	return &brokerFactory{
		logger: logger,
	}
}

type brokerFactory struct {
	logger *zap.Logger
}

func (h *brokerFactory) Broker(c ReadWriteFlusher) (Broker, error) {
	return newBroker(h.logger, c), nil
}

func newBroker(logger *zap.Logger, c ReadWriteFlusher) *broker {
	p := &broker{
		logger:       logger,
		c:            c,
		localParams:  make(chan brokerLocalParams, 1),
		initRequired: make(chan struct{}, 1),
		initDone:     make(chan error, 1),
		keys:         make(chan [][]byte, 1),
	}

	go func() {
		if err := p.readInits(); err != nil {
			log.Println(err)
		}
	}()

	return p
}

type brokerLocalParams struct {
	preferSender bool
	keys         [][]byte
}

type broker struct {
	logger       *zap.Logger
	cLock        sync.Mutex
	c            ReadWriteFlusher
	localParams  chan brokerLocalParams
	initLock     sync.Mutex
	initRequired chan struct{}
	initDone     chan error
	keys         chan [][]byte
	closeOnce    sync.Once
}

func (p *broker) Init(preferSender bool, keys [][]byte) error {
	go func() {
		p.initLock.Lock()
		defer p.initLock.Unlock()

		p.logger.Debug("starting network negotiation", zap.Int("keys", len(keys)))

		p.localParams <- brokerLocalParams{preferSender, keys}

		if err := p.sendInit(keys); err != nil {
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

func (p *broker) InitRequired() <-chan struct{} {
	return p.initRequired
}

func (p *broker) Keys() <-chan [][]byte {
	return p.keys
}

func (p *broker) Close() {
	p.closeOnce.Do(func() {
		close(p.localParams)
		close(p.initRequired)
		close(p.initDone)
		close(p.keys)
	})
}

func (p *broker) readInits() (err error) {
	defer p.Close()

	for {
		var handshake pb.NetworkHandshake
		if err := protoutil.ReadStream(p.c, &handshake); err != nil {
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

func (p *broker) sendInit(keys [][]byte) error {
	p.cLock.Lock()
	defer p.cLock.Unlock()

	err := protoutil.WriteStream(p.c, &pb.NetworkHandshake{
		Body: &pb.NetworkHandshake_Init_{
			Init: &pb.NetworkHandshake_Init{
				KeyCount: int32(len(keys)),
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

func (p *broker) awaitLocalParams() (bool, [][]byte, error) {
	var l brokerLocalParams
	var ok bool

	select {
	case l, ok = <-p.localParams:
	default:
		p.initRequired <- struct{}{}

		select {
		case l, ok = <-p.localParams:
		case <-time.After(10 * time.Second):
			return false, nil, errors.New("timeout")
		}
	}

	if !ok {
		return false, nil, errors.New("broker closed")
	}
	return l.preferSender, l.keys, nil
}

func (p *broker) handleInit(init *pb.NetworkHandshake_Init) error {
	preferSender, keys, err := p.awaitLocalParams()
	if err != nil {
		return err
	}

	// communication cost for the PSZ sender scales better than the receiver.
	if int(init.KeyCount) < len(keys) || (int(init.KeyCount) == len(keys) && preferSender) {
		return p.exchangeKeysAsSender(keys)
	}

	keys, err = p.exchangeKeysAsReceiver(keys)
	if err != nil {
		return err
	}

	p.keys <- keys
	return err
}

func (p *broker) exchangeKeysAsSender(keys [][]byte) error {
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

func (p *broker) exchangeKeysAsReceiver(keys [][]byte) ([][]byte, error) {
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

func newRNG() (*mpc.AESRNG, error) {
	var seed [16]byte
	if _, err := rand.Read(seed[:]); err != nil {
		return nil, err
	}
	return mpc.NewAESRNG(seed[:])
}
