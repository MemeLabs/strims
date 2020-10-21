// +build js

package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// StartHLSEgress ...
func (s *videoService) StartHLSEgress(ctx context.Context, r *pb.StartHLSEgressRequest) (*pb.StartHLSEgressResponse, error) {
	return &pb.StartHLSEgressResponse{}, ErrMethodNotImplemented
}

// StopHLSEgress ...
func (s *videoService) StopHLSEgress(ctx context.Context, r *pb.StopHLSEgressRequest) (*pb.StopHLSEgressResponse, error) {
	return &pb.StopHLSEgressResponse{}, ErrMethodNotImplemented
}

// StartRTMPIngress ...
func (s *videoService) StartRTMPIngress(ctx context.Context, r *pb.StartRTMPIngressRequest) (*pb.StartRTMPIngressResponse, error) {
	return &pb.StartRTMPIngressResponse{}, ErrMethodNotImplemented
}
