package network

import (
	"crypto/rand"

	"github.com/MemeLabs/go-ppspp/pkg/mpc"
	"go.uber.org/zap"
)

// NewBroker constructs a new local broker factory
func NewBroker(logger *zap.Logger) Broker {
	return &broker{
		logger: logger,
	}
}

type broker struct {
	logger *zap.Logger
}

func (p *broker) SendKeys(c ReadWriteFlusher, keys [][]byte) error {
	rng, err := newRNG()
	if err != nil {
		return err
	}
	ot, err := mpc.NewChaoOrlandiSender(c, rng)
	if err != nil {
		return err
	}
	ote, err := mpc.NewKOSReceiver(c, ot, rng)
	if err != nil {
		return err
	}
	oprf, err := mpc.NewKKRTSender(c, ote, rng)
	if err != nil {
		return err
	}
	psi, err := mpc.NewPSZSender(oprf)
	if err != nil {
		return err
	}
	err = psi.Send(c, keys, rng)
	if err != nil {
		return err
	}

	return nil
}

func (p *broker) ReceiveKeys(c ReadWriteFlusher, keys [][]byte) ([][]byte, error) {
	rng, err := newRNG()
	if err != nil {
		return nil, err
	}
	ot, err := mpc.NewChaoOrlandiReceiver(c, rng)
	if err != nil {
		return nil, err
	}
	ote, err := mpc.NewKOSSender(c, ot, rng)
	if err != nil {
		return nil, err
	}
	oprf, err := mpc.NewKKRTReceiver(c, ote, rng)
	if err != nil {
		return nil, err
	}
	psi, err := mpc.NewPSZReceiver(oprf)
	if err != nil {
		return nil, err
	}
	results, err := psi.Receive(c, keys, rng)
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
