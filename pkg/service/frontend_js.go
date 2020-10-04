// +build js

package service

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// StartHLSEgress ...
func (s *Frontend) StartHLSEgress(ctx context.Context, r *pb.StartHLSEgressRequest) (*pb.StartHLSEgressResponse, error) {
	return &pb.StartHLSEgressResponse{}, errors.New("not implemented")
}

// StopHLSEgress ...
func (s *Frontend) StopHLSEgress(ctx context.Context, r *pb.StopHLSEgressRequest) (*pb.StopHLSEgressResponse, error) {
	return &pb.StopHLSEgressResponse{}, errors.New("not implemented")
}

// StartRTMPIngress ...
func (s *Frontend) StartRTMPIngress(ctx context.Context, r *pb.StartRTMPIngressRequest) (*pb.StartRTMPIngressResponse, error) {
	return &pb.StartRTMPIngressResponse{}, errors.New("not implemented")
}
