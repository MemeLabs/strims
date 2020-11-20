package ca

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var caSalt = []byte("ca")

const clientTimeout = time.Second * 5

// NewServer ...
func NewServer(ctx context.Context, logger *zap.Logger, client *vpn.Client, network *pb.Network) (*Server, error) {
	ctx, cancel := context.WithCancel(ctx)

	ca := &Server{
		logger:  logger,
		network: network,
		cancel:  cancel,
	}

	server := rpc.NewServer(logger)
	api.RegisterCAService(server, ca)

	go server.Listen(ctx, &rpc.VPNServerDialer{
		Logger: logger,
		Client: client,
		Key:    network.Key,
		Salt:   caSalt,
	})

	return ca, nil
}

// Server ...
type Server struct {
	logger  *zap.Logger
	lock    sync.Mutex
	network *pb.Network
	cancel  context.CancelFunc

	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

// Close ...
func (s *Server) Close() {
	s.cancel()
}

// UpdateNetwork ...
func (s *Server) UpdateNetwork(network *pb.Network) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.network = network
}

// Renew ...
func (s *Server) Renew(ctxt context.Context, req *pb.CARenewRequest) (*pb.CARenewResponse, error) {
	s.lock.Lock()
	defer s.lock.Unlock()

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

// NewClient ...
func NewClient(logger *zap.Logger, client *vpn.Client) (*Client, error) {
	rpcClient, err := rpc.NewClient(logger, &rpc.VPNDialer{
		Logger: logger,
		Client: client,
		Key:    client.Network.Key(),
		Salt:   caSalt,
	})
	if err != nil {
		return nil, err
	}

	return &Client{rpcClient, api.NewCAClient(rpcClient)}, nil
}

// Client ...
type Client struct {
	rpc *rpc.Client
	*api.CAClient
}

// Close ...
func (c *Client) Close() {
	c.rpc.Close()
}
