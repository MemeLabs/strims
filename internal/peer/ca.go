package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
)

type caService struct {
	Peer app.Peer
	App  app.Control
}

func (s *caService) Renew(ctx context.Context, req *ca.CAPeerRenewRequest) (*ca.CAPeerRenewResponse, error) {
	cert, err := s.App.Network().CA().ForwardRenewRequest(ctx, req.Certificate, req.CertificateRequest)
	if err != nil {
		return nil, err
	}
	return &ca.CAPeerRenewResponse{Certificate: cert}, nil
}
