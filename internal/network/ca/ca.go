// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"context"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network/dialer"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"go.uber.org/zap"
)

// NewCA ...
func NewCA(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	dialer *dialer.Dialer,
	transfer transfer.Control,
) *CA {
	return &CA{
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
type CA struct {
	ctx       context.Context
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	dialer    *dialer.Dialer
	transfer  transfer.Control
	events    chan any

	lock    sync.Mutex
	runners hashmap.Map[[]byte, *runner]
}

// Run ...
func (t *CA) Run() {
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

func (t *CA) handleNetworkStart(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	r, err := newRunner(t.ctx, t.logger, t.store, t.observers, t.dialer, t.transfer, network)
	if err != nil {
		t.logger.Error("failed to start directory runner", zap.Error(err))
		return
	}
	t.runners.Set(dao.NetworkKey(network), r)
}

func (t *CA) handleNetworkStop(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Delete(dao.NetworkKey(network)); ok {
		r.Close()
	}
}

func (t *CA) handleNetworkChange(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	if r, ok := t.runners.Get(dao.NetworkKey(network)); ok {
		r.Sync(network)
	}
}

// ForwardRenewRequest ...
func (t *CA) ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error) {
	networkKey := dao.CertificateNetworkKey(cert)
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

func (t *CA) find(ctx context.Context, networkKey []byte, req *networkv1ca.CAFindRequest) (*certificate.Certificate, error) {
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

func (t *CA) FindBySubject(ctx context.Context, networkKey []byte, subject string) (*certificate.Certificate, error) {
	return t.find(ctx, networkKey, &networkv1ca.CAFindRequest{Query: &networkv1ca.CAFindRequest_Subject{Subject: subject}})
}

func (t *CA) FindByKey(ctx context.Context, networkKey []byte, key []byte) (*certificate.Certificate, error) {
	return t.find(ctx, networkKey, &networkv1ca.CAFindRequest{Query: &networkv1ca.CAFindRequest_Key{Key: key}})
}
