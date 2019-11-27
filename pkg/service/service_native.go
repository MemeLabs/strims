// +build !js

package service

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
)

// StartHLSIngress ...
func (s *Service) StartHLSIngress(r *StartHLSIngressRequest) (*StartHLSIngressResponse, error) {
	if s.ingress != nil {
		return nil, errors.New("hls ingress already started")
	}

	s.ingress = lhls.NewIngress(context.TODO(), s.h)
	go s.ingress.ListenAndServe()

	return &StartHLSIngressResponse{}, nil
}

// GetIngressStreams ...
func (s *Service) GetIngressStreams(r *GetIngressStreamsRequest) (chan *GetIngressStreamsResponse, error) {
	res := make(chan *GetIngressStreamsResponse, 1)

	// go func() {
	// 	s.ingress := lhls.NewIngress(context.TODO(), s.h)
	// 	go s.ingress.ListenAndServe()

	// 	for s := range s.ingress.DebugSwarms {
	// 		res <- &GetIngressStreamsResponse{
	// 			SwarmUri: s.ID.String(),
	// 		}
	// 	}
	// }()

	return res, nil
}

// StartHLSEgress ...
func (s *Service) StartHLSEgress(r *StartHLSEgressRequest) (*StartHLSEgressResponse, error) {
	if s.egress != nil {
		return nil, errors.New("hls egress already started")
	}

	srv := lhls.NewEgress()
	go srv.ListenAndServe()

	return &StartHLSEgressResponse{}, nil
}

// StopHLSEgress ...
func (s *Service) StopHLSEgress(r *StopHLSEgressRequest) (*StopHLSEgressResponse, error) {
	if s.egress != nil {
		return nil, errors.New("hls egress is not running")
	}

	err := s.egress.Close()
	s.egress = nil

	return &StopHLSEgressResponse{}, err
}
