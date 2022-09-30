// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ca

import (
	"context"

	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
)

type peerService struct {
	ca *CA
}

func (s *peerService) Renew(ctx context.Context, req *networkv1ca.CAPeerRenewRequest) (*networkv1ca.CAPeerRenewResponse, error) {
	cert, err := s.ca.ForwardRenewRequest(ctx, req.Certificate, req.CertificateRequest)
	if err != nil {
		return nil, err
	}
	return &networkv1ca.CAPeerRenewResponse{Certificate: cert}, nil
}
