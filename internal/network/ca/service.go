package ca

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
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
		done:        make(chan struct{}),
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
	closeOnce   sync.Once
	done        chan struct{}
	eventWriter *protoutil.ChunkStreamWriter

	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

func (s *service) Run(ctx context.Context) error {
	defer s.Close()

	events := make(chan interface{}, 8)
	s.observers.Notify(events)
	defer s.observers.StopNotifying(events)

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case *networkv1.NetworkChangeEvent:
				s.network.Swap(e.Network)
			}
		case <-s.done:
			return errors.New("closed")
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

// Close ...
func (s *service) Close() {
	s.closeOnce.Do(func() {
		s.logCache.Close()
		close(s.done)
	})
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
		return nil, err
	}
	s.logCache.Store(log)

	return &networkv1ca.CARenewResponse{Certificate: cert}, nil
}

// Find ...
func (s *service) Find(ctx context.Context, req *networkv1ca.CAFindRequest) (*networkv1ca.CAFindResponse, error) {
	if req.Subject == "" && req.SerialNumber == nil {
		return nil, errors.New("find request must specify subject or serial number")
	}

	var log *networkv1ca.CertificateLog
	var err error
	if req.Subject != "" {
		log, err = s.logCache.BySubject.Get(dao.FormatCertificateLogSubjectKey(s.network.Get().Id, req.Subject))
	} else {
		log, err = s.logCache.BySerialNumber.Get(dao.FormatCertificateLogsSerialNumberKey(s.network.Get().Id, req.SerialNumber))
	}
	if err != nil {
		return nil, err
	}

	if req.FullChain {
		cert := log.Certificate
		for cert.GetParentSerialNumber() != nil {
			parentLog, err := s.logCache.BySerialNumber.Get(dao.FormatCertificateLogsSerialNumberKey(s.network.Get().Id, cert.GetParentSerialNumber()))
			if err != nil {
				return nil, err
			}
			cert.ParentOneof = &certificate.Certificate_Parent{Parent: parentLog.Certificate}
			cert = parentLog.Certificate
		}
	}

	return &networkv1ca.CAFindResponse{Certificate: log.Certificate}, nil
}
