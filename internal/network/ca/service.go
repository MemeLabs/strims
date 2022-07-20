// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"bytes"
	"context"
	"errors"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	networkv1errors "github.com/MemeLabs/strims/pkg/apis/network/v1/errors"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"go.uber.org/zap"
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
	store *dao.ProfileStore,
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
	store     *dao.ProfileStore
	observers *event.Observers

	logCache    dao.CertificateLogCache
	network     syncutil.Pointer[networkv1.Network]
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
				if e.Network.Id == s.network.Get().Id {
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

// Renew ...
func (s *service) Renew(ctx context.Context, req *networkv1ca.CARenewRequest) (*networkv1ca.CARenewResponse, error) {
	if err := dao.VerifyCertificate(req.Certificate); err != nil {
		return nil, err
	}
	if !bytes.Equal(dao.CertificateNetworkKey(req.Certificate), dao.NetworkKey(s.network.Get())) {
		return nil, errors.New("signing certificate network key mismatch")
	}
	if !bytes.Equal(req.Certificate.GetKey(), req.CertificateRequest.GetKey()) {
		return nil, errors.New("signing certificate key does not match certificate request key")
	}

	// TODO: record invite graph

	err := dao.VerifyCertificateRequest(req.CertificateRequest, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}

	// TODO: verify invitation policy
	// TODO: verify nick
	cert, err := dao.SignCertificateRequestWithNetwork(req.CertificateRequest, s.network.Get().GetServerConfig())
	if err != nil {
		return nil, err
	}

	log, err := dao.NewCertificateLog(s.store, s.network.Get().Id, cert)
	if err != nil {
		return nil, err
	}
	err = dao.CertificateLogs.Insert(s.store, log)
	if err != nil {
		if errors.Is(err, dao.ErrCertificateSubjectInUse) {
			return nil, rpc.WrapError(err, networkv1errors.ErrorCode_CERTIFICATE_SUBJECT_IN_USE)
		}
		return nil, err
	}
	s.logCache.Store(log)

	return &networkv1ca.CARenewResponse{Certificate: cert}, nil
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
		log, err = s.logCache.BySubject.Get(dao.FormatCertificateLogSubjectKey(s.network.Get().Id, q.Subject))
	case *networkv1ca.CAFindRequest_SerialNumber:
		log, err = s.logCache.BySerialNumber.Get(dao.FormatCertificateLogSerialNumberKey(s.network.Get().Id, q.SerialNumber))
	case *networkv1ca.CAFindRequest_Key:
		log, err = s.logCache.ByKey.Get(dao.FormatCertificateLogKeyKey(s.network.Get().Id, q.Key))
	}
	if err != nil {
		return nil, err
	}

	if req.FullChain {
		cert := log.Certificate
		for cert.GetParentSerialNumber() != nil {
			parentLog, err := s.logCache.BySerialNumber.Get(dao.FormatCertificateLogSerialNumberKey(s.network.Get().Id, cert.GetParentSerialNumber()))
			if err != nil {
				return nil, err
			}
			cert.ParentOneof = &certificate.Certificate_Parent{Parent: parentLog.Certificate}
			cert = parentLog.Certificate
		}
	}

	return &networkv1ca.CAFindResponse{Certificate: log.Certificate}, nil
}
