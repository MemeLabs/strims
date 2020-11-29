package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type caService struct {
	Peer *app.Peer
	App  *app.Control
}

func (s *caService) Renew(ctx context.Context, req *pb.CAPeerRenewRequest) (*pb.CAPeerRenewResponse, error) {
	cert, err := s.App.CA().ForwardRenewRequest(ctx, req.Certificate, req.CertificateRequest)
	if err != nil {
		return nil, err
	}
	return &pb.CAPeerRenewResponse{Certificate: cert}, nil
}