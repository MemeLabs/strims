package service

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

// New ...
func New(h *encoding.Host) *Service {
	return &Service{
		h: h,
	}
}

// Service ...
type Service struct {
	h       *encoding.Host
	ingress *lhls.Ingress
	egress  *lhls.Egress
}

// JoinSwarm ...
func (s *Service) JoinSwarm(ctx context.Context, r *JoinSwarmRequest) (*JoinSwarmResponse, error) {
	return &JoinSwarmResponse{}, nil
}

// LeaveSwarm ...
func (s *Service) LeaveSwarm(ctx context.Context, r *LeaveSwarmRequest) (*LeaveSwarmResponse, error) {
	return &LeaveSwarmResponse{}, nil
}

// BootstrapDHT ...
func (s *Service) BootstrapDHT(ctx context.Context, r *BootstrapDHTRequest) (*BootstrapDHTResponse, error) {
	return &BootstrapDHTResponse{}, nil
}
