package network

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/services/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var caSalt = []byte("ca:addr")

const clientTimeout = time.Second * 5

// NewCA ...
func NewCA(ctx context.Context, logger *zap.Logger, client *vpn.Client, network *pb.Network) (*CA, error) {
	ca := &CA{
		logger:  logger,
		network: network,
	}

	host, err := rpc.NewHost(logger, client, network.Key, caSalt)
	if err != nil {
		return nil, err
	}

	api.RegisterCAService(host, ca)

	return ca, nil
}

// CA ...
type CA struct {
	logger  *zap.Logger
	network *pb.Network

	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

// Renew ...
func (s *CA) Renew(ctxt context.Context, req *pb.CARenewRequest) (*pb.CARenewResponse, error) {
	err := dao.VerifyCertificateRequest(req.CertificateRequest, pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}

	// TODO: check subject (nick) availability
	// TODO: verify invitation policy

	networkCert, err := dao.NewSelfSignedCertificate(s.network.Key, pb.KeyUsage_KEY_USAGE_SIGN, time.Hour*24, dao.WithSubject(s.network.Name))
	if err != nil {
		return nil, err
	}

	cert, err := dao.SignCertificateRequest(req.CertificateRequest, time.Hour*24, s.network.Key)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: networkCert}

	return &pb.CARenewResponse{Certificate: cert}, nil
}

// NewCAClient ...
func NewCAClient(logger *zap.Logger, client *vpn.Client) (*api.CAClient, error) {
	key := dao.GetRootCert(client.Network.Certificate()).Key
	c, err := rpc.NewClient(logger, client, key, caSalt)
	if err != nil {
		return nil, err
	}

	return api.NewCAClient(c), nil
}
