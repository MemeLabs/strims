// +build !js

package service

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

// StartHLSIngress ...
func (s *Service) StartHLSIngress(ctx context.Context, r *StartHLSIngressRequest) (*StartHLSIngressResponse, error) {
	if s.ingress != nil {
		return nil, errors.New("hls ingress already started")
	}

	s.ingress = lhls.NewIngress(context.TODO(), s.h)
	go s.ingress.ListenAndServe()

	return &StartHLSIngressResponse{}, nil
}

// GetIngressStreams ...
func (s *Service) GetIngressStreams(ctx context.Context, r *GetIngressStreamsRequest) (chan *GetIngressStreamsResponse, error) {
	res := make(chan *GetIngressStreamsResponse, 1)

	go func() {
		swarms := make(chan *encoding.Swarm, 16)
		s.ingress.Notify(swarms)
		defer s.ingress.Stop(swarms)

		for swarm := range swarms {
			res <- &GetIngressStreamsResponse{
				SwarmUri: swarm.ID.String(),
			}
		}
	}()

	return res, nil
}

// StartHLSEgress ...
func (s *Service) StartHLSEgress(ctx context.Context, r *StartHLSEgressRequest) (*StartHLSEgressResponse, error) {
	if s.egress != nil {
		return nil, errors.New("hls egress already started")
	}

	s.egress = lhls.NewEgress(lhls.DefaultEgressOptions)
	go s.egress.ListenAndServe()

	return &StartHLSEgressResponse{}, nil
}

// StopHLSEgress ...
func (s *Service) StopHLSEgress(ctx context.Context, r *StopHLSEgressRequest) (*StopHLSEgressResponse, error) {
	if s.egress != nil {
		return nil, errors.New("hls egress is not running")
	}

	err := s.egress.Close()
	s.egress = nil

	return &StopHLSEgressResponse{}, err
}
