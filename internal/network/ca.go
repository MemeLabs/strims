package network

import (
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"go.uber.org/zap"
)

var _ CA = &ca{}

type CA interface {
	ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error)
	FindBySubject(ctx context.Context, networkKey []byte, subject string) (*certificate.Certificate, error)
}

// newCA ...
func newCA(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer Dialer,
	transfer transfer.Control,
) *ca {
	return &ca{
		ctx:       ctx,
		logger:    logger,
		store:     store,
		observers: observers,
		dialer:    dialer,
		transfer:  transfer,
		events:    observers.Chan(),
		runners:   hashmap.New[[]byte, *runner](hashmap.NewByteInterface[[]byte]()),
	}
}

// CA ...
type ca struct {
	ctx       context.Context
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    Dialer
	transfer  transfer.Control
	events    chan interface{}

	lock    sync.Mutex
	runners hashmap.Map[[]byte, *runner]
}

// Run ...
func (t *ca) Run() {
	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.NetworkStart:
				t.handleNetworkStart(e.Network)
			case event.NetworkStop:
				t.handleNetworkStop(e.Network)
			case *networkv1.NetworkChangeEvent:
				t.handleNetworkChange(e.Network)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func (t *ca) handleNetworkStart(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	r, err := newRunner(t.ctx, t.logger, t.store, t.observers, t.dialer, t.transfer, network)
	if err != nil {
		t.logger.Error("failed to start directory runner", zap.Error(err))
		return
	}
	t.runners.Set(dao.NetworkKey(network), r)
}

func (t *ca) handleNetworkStop(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Delete(dao.NetworkKey(network)); ok {
		r.Close()
	}
}

func (t *ca) handleNetworkChange(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Get(dao.NetworkKey(network)); ok {
		r.Sync(network)
	}
}

// ForwardRenewRequest ...
func (t *ca) ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
	networkKey := networkKeyForCertificate(cert)
	client, err := t.dialer.Client(ctx, networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, err
	}
	defer client.Close()

	renewReq := &networkv1ca.CARenewRequest{
		Certificate:        cert,
		CertificateRequest: csr,
	}
	renewRes := &networkv1ca.CARenewResponse{}
	if err := networkv1ca.NewCAClient(client).Renew(ctx, renewReq, renewRes); err != nil {
		return nil, err
	}

	return renewRes.Certificate, nil
}

func (t *ca) find(ctx context.Context, networkKey []byte, req *networkv1ca.CAFindRequest) (*certificate.Certificate, error) {
	client, err := t.dialer.Client(ctx, networkKey, networkKey, AddressSalt)
	if err != nil {
		return nil, err
	}
	defer client.Close()

	res := &networkv1ca.CAFindResponse{}
	if err := networkv1ca.NewCAClient(client).Find(ctx, req, res); err != nil {
		return nil, err
	}

	return res.Certificate, nil
}

func (t *ca) FindBySubject(ctx context.Context, networkKey []byte, subject string) (*certificate.Certificate, error) {
	return t.find(ctx, networkKey, &networkv1ca.CAFindRequest{Subject: subject})
}
