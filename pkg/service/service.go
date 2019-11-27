package service

import (
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
func (s *Service) JoinSwarm(r *JoinSwarmRequest) (*JoinSwarmResponse, error) {
	return &JoinSwarmResponse{}, nil
}

// LeaveSwarm ...
func (s *Service) LeaveSwarm(r *LeaveSwarmRequest) (*LeaveSwarmResponse, error) {
	return &LeaveSwarmResponse{}, nil
}
