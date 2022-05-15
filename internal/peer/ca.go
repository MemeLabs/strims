// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package peer

import (
	"context"

	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
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
