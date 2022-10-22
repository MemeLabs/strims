// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	networkv1errors "github.com/MemeLabs/strims/pkg/apis/network/v1/errors"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
)

const (
	aliasChangeCooldown      = 30 * 24 * time.Hour
	aliasReservationCooldown = 180 * 24 * time.Hour
)

// AddressSalt ...
var AddressSalt = []byte("ca")

var caSwarmOptions = ppspp.SwarmOptions{
	ChunkSize:  1024,
	LiveWindow: 256,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
	DeliveryMode: ppspp.BestEffortDeliveryMode,
}

// New ...
func newCAService(
	logger *zap.Logger,
	store dao.Store,
	observers *event.Observers,
	network *networkv1.Network,
	ew *protoutil.ChunkStreamWriter,
) *service {
	return &service{
		logger:    logger,
		store:     store,
		observers: observers,

		logCache:    dao.NewCertificateLogCache(store, nil),
		network:     syncutil.NewPointer(network),
		eventWriter: ew,
	}
}

// service ...
type service struct {
	logger    *zap.Logger
	store     dao.Store
	observers *event.Observers

	logCache    dao.CertificateLogCache
	network     atomic.Pointer[networkv1.Network]
	eventWriter *protoutil.ChunkStreamWriter

	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

func (s *service) Run(ctx context.Context) error {
	defer s.Close()

	events, done := s.observers.Events()
	defer done()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *networkv1.NetworkChangeEvent:
				if e.Network.Id == s.network.Load().Id {
					s.network.Swap(e.Network)
				}
			}
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

// Close ...
func (s *service) Close() {
	s.logCache.Close()
}

func (s *service) networkID() uint64 {
	return s.network.Load().Id
}

// Renew ...
func (s *service) Renew(ctx context.Context, req *networkv1ca.CARenewRequest) (*networkv1ca.CARenewResponse, error) {
	if err := dao.VerifyCertificate(req.Certificate); err != nil {
		return nil, err
	}
	if !bytes.Equal(dao.CertificateNetworkKey(req.Certificate), dao.NetworkKey(s.network.Load())) {
		return nil, errors.New("signing certificate network key mismatch")
	}
	if !bytes.Equal(req.Certificate.GetKey(), req.CertificateRequest.GetKey()) {
		return nil, errors.New("signing certificate key does not match certificate request key")
	}

	err := dao.VerifyCertificateRequest(req.CertificateRequest, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}

	cert, err := dao.SignCertificateRequestWithNetwork(req.CertificateRequest, s.network.Load().GetServerConfig())
	if err != nil {
		return nil, err
	}

	log, err := dao.NewCertificateLog(s.store, s.networkID(), cert)
	if err != nil {
		return nil, err
	}

	err = s.store.Update(func(tx kv.RWTx) error {
		peer, err := dao.NetworkPeersByPublicKey.Get(tx, dao.FormatNetworkPeerPublicKeyKey(s.networkID(), cert.Key))
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}

		if peer != nil {
			if err := s.updatePeer(tx, peer, cert); err != nil {
				return err
			}
		} else {
			chain := dao.CertificateChain(req.Certificate)
			if len(chain) != 4 {
				return errors.New("malformed invite certificate chain")
			}
			if err := s.insertPeer(tx, cert, chain[2].Key); err != nil {
				return err
			}
		}

		if err := dao.CertificateLogs.Insert(tx, log); err != nil {
			return err
		}
		s.logCache.Store(log)
		return nil
	})
	if err != nil {
		return nil, err
	}

	return &networkv1ca.CARenewResponse{Certificate: cert}, nil
}

func (s *service) insertPeer(tx kv.RWTx, cert *certificate.Certificate, inviterKey []byte) error {
	if err := s.reserveAlias(tx, timeutil.Now(), cert); err != nil {
		return err
	}

	ip, err := dao.NetworkPeersByPublicKey.Get(tx, dao.FormatNetworkPeerPublicKeyKey(s.networkID(), inviterKey))
	if err != nil {
		return fmt.Errorf("inviter not found: %w", err)
	}
	if ip.IsBanned {
		return rpc.WrapError(errors.New("inviter banned"), networkv1errors.ErrorCode_INVITER_BANNED)
	}
	if ip.InviteQuota == 0 {
		return rpc.WrapError(errors.New("invitation quota exceeded"), networkv1errors.ErrorCode_INVITATION_QUOTA_EXCEEDED)
	}
	ip.InviteQuota--
	if err := dao.NetworkPeers.Update(tx, ip); err != nil {
		return err
	}

	p, err := dao.NewNetworkPeer(dao.ProfileID.IDGenerator(tx), s.networkID(), cert.Key, cert.Subject, ip.Id)
	if err != nil {
		return err
	}
	return dao.NetworkPeers.Insert(tx, p)
}

func (s *service) updatePeer(tx kv.RWTx, peer *networkv1.Peer, cert *certificate.Certificate) error {
	if peer.Alias == cert.Subject {
		return nil
	}

	now := timeutil.Now()

	if !now.After(timeutil.Unix(peer.AliasChangedAt, 0).Add(aliasChangeCooldown)) {
		return rpc.WrapError(errors.New("alias was changed too recently"), networkv1errors.ErrorCode_ALIAS_CHANGE_COOLDOWN_VIOLATIED)
	}

	if err := s.reserveAlias(tx, now, cert); err != nil {
		return err
	}

	rid, err := dao.NetworkAliasReservationsByAlias.GetID(tx, dao.FormatNetworkAliasReservationAliasKey(s.networkID(), peer.Alias))
	if err != nil {
		return err
	}
	_, err = dao.NetworkAliasReservations.Transform(tx, rid, func(p *networkv1.AliasReservation) error {
		p.PeerKey = nil
		p.ReservedUntil = now.Add(aliasReservationCooldown).Unix()
		return nil
	})
	if err != nil {
		return err
	}

	peer.Alias = cert.Subject
	peer.AliasChangedAt = now.Unix()
	return dao.NetworkPeers.Update(tx, peer)
}

func (s *service) reserveAlias(tx kv.RWTx, now timeutil.Time, cert *certificate.Certificate) error {
	r, err := dao.NetworkAliasReservationsByAlias.Get(tx, dao.FormatNetworkAliasReservationAliasKey(s.networkID(), cert.Subject))
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return err
	}
	if r == nil {
		r, err = dao.NewNetworkAliasReservation(dao.ProfileID.IDGenerator(tx), s.networkID(), cert.Subject, cert.Key)
		if err != nil {
			return err
		}
		return dao.NetworkAliasReservations.Insert(tx, r)
	} else if !bytes.Equal(r.PeerKey, cert.Key) {
		if !now.After(timeutil.Unix(r.ReservedUntil, 0)) {
			return rpc.WrapError(errors.New("alias in use"), networkv1errors.ErrorCode_ALIAS_IN_USE)
		}
		r.PeerKey = cert.Key
		r.ReservedUntil = timeutil.MaxTime.Unix()
		return dao.NetworkAliasReservations.Update(tx, r)
	}
	return nil
}

// Find ...
func (s *service) Find(ctx context.Context, req *networkv1ca.CAFindRequest) (*networkv1ca.CAFindResponse, error) {
	if req.Query == nil {
		return nil, errors.New("find request must specify subject or serial number")
	}

	var log *networkv1ca.CertificateLog
	var err error
	switch q := req.Query.(type) {
	case *networkv1ca.CAFindRequest_Subject:
		log, err = s.logCache.BySubject.Get(dao.FormatCertificateLogSubjectKey(s.network.Load().Id, q.Subject))
	case *networkv1ca.CAFindRequest_SerialNumber:
		log, err = s.logCache.BySerialNumber.Get(dao.FormatCertificateLogSerialNumberKey(s.network.Load().Id, q.SerialNumber))
	case *networkv1ca.CAFindRequest_Key:
		log, err = s.logCache.ByKey.Get(dao.FormatCertificateLogKeyKey(s.network.Load().Id, q.Key))
	}
	if err != nil {
		return nil, err
	}

	if req.FullChain {
		cert := log.Certificate
		for cert.GetParentSerialNumber() != nil {
			parentLog, err := s.logCache.BySerialNumber.Get(dao.FormatCertificateLogSerialNumberKey(s.network.Load().Id, cert.GetParentSerialNumber()))
			if err != nil {
				return nil, err
			}
			cert.ParentOneof = &certificate.Certificate_Parent{Parent: parentLog.Certificate}
			cert = parentLog.Certificate
		}
	}

	return &networkv1ca.CAFindResponse{Certificate: log.Certificate}, nil
}
