package ca

import (
	"context"
	"sync"
	"time"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"go.uber.org/zap"
)

// AddressSalt ...
var AddressSalt = []byte("ca")

const clientTimeout = time.Second * 5
const certificateValidDuration = time.Hour * 24 * 14

// NewService ...
func NewService(logger *zap.Logger, network *network.Network) *Service {
	return &Service{
		logger:  logger,
		network: network,
	}
}

// Service ...
type Service struct {
	logger  *zap.Logger
	lock    sync.Mutex
	network *network.Network
	cancel  context.CancelFunc

	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

// UpdateNetwork ...
func (s *Service) UpdateNetwork(network *network.Network) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.network = network
}

// Renew ...
func (s *Service) Renew(ctxt context.Context, req *ca.CARenewRequest) (*ca.CARenewResponse, error) {
	s.lock.Lock()
	defer s.lock.Unlock()

	err := dao.VerifyCertificateRequest(req.CertificateRequest, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}

	// TODO: check subject (nick) availability
	// TODO: verify invitation policy
	networkCert, err := dao.NewSelfSignedCertificate(s.network.Key, certificate.KeyUsage_KEY_USAGE_SIGN, certificateValidDuration, dao.WithSubject(s.network.Name))
	if err != nil {
		return nil, err
	}

	cert, err := dao.SignCertificateRequest(req.CertificateRequest, certificateValidDuration, s.network.Key)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: networkCert}

	return &ca.CARenewResponse{Certificate: cert}, nil
}
