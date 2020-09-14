// +build !js

package service

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/hls"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"go.uber.org/zap"
)

const egressAddr = "127.0.0.1:0"

type egress struct {
	listener net.Listener
}

// StartHLSEgress ...
func (s *Frontend) StartHLSEgress(ctx context.Context, r *pb.StartHLSEgressRequest) (*pb.StartHLSEgressResponse, error) {
	session := ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	tif, _ := session.Values.Load(r.VideoId)
	t, ok := tif.(*VideoClient)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	stream := hls.NewStream(hls.DefaultStreamOptions)
	go func() {
		if err := t.SendStream(context.TODO(), stream); err != nil {
			s.logger.Debug("sending stream to hls egress failed", zap.Error(err))
		}
	}()

	svc := hls.NewService()
	svc.InsertChannel(&hls.Channel{
		Name:   "live",
		Stream: stream,
	})

	addr := egressAddr
	if r.Address != "" {
		addr = r.Address
	}
	lis, err := net.Listen("tcp", addr)
	if err != nil {
		return nil, err
	}

	go func() {
		srv := &http.Server{
			Handler: svc.Handler(),
		}
		if err := srv.Serve(lis); err != nil {
			s.logger.Debug("failed", zap.Error(err))
		}
	}()

	id := session.Store(&egress{
		listener: lis,
	})

	return &pb.StartHLSEgressResponse{
		Id:  id,
		Url: fmt.Sprintf("http://%s/hls/live/index.m3u8", lis.Addr().String()),
	}, nil
}

// StopHLSEgress ...
func (s *Frontend) StopHLSEgress(ctx context.Context, r *pb.StopHLSEgressRequest) (*pb.StopHLSEgressResponse, error) {
	return &pb.StopHLSEgressResponse{}, nil
}

// StartRTMPIngress ...
func (s *Frontend) StartRTMPIngress(ctx context.Context, r *pb.StartRTMPIngressRequest) (*pb.StartRTMPIngressResponse, error) {
	session := ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	x := rtmpingress.NewTranscoder(s.logger)
	rtmp := rtmpingress.Server{
		Addr: ":1971",
		HandleStream: func(a *rtmpingress.StreamAddr, c *rtmpingress.Conn) {
			s.logger.Debug("rtmp stream opened", zap.String("key", a.Key))

			v, err := NewVideoServer(s.logger)
			if err != nil {
				s.logger.Debug("starting video server failed", zap.Error(err))
				if err := c.Close(); err != nil {
					s.logger.Debug("closing rtmp net con failed", zap.Error(err))
				}
				return
			}

			go func() {
				if err := x.Transcode(a.URI, a.Key, "source", v); err != nil {
					s.logger.Debug("transcoder finished", zap.Error(err))
				}
			}()

			memberships, err := dao.GetNetworkMemberships(session.ProfileStore())
			if err != nil {
				s.logger.Debug("loading network memberships failed", zap.Error(err))

				v.Stop()

				if err := c.Close(); err != nil {
					s.logger.Debug("closing rtmp net con failed", zap.Error(err))
				}
				return
			}

			for _, membership := range memberships {
				membership := membership
				go func() {
					svc, ok := ctl.NetworkServices(dao.GetRootCert(membership.Certificate).Key)
					if !ok {
						s.logger.Debug("publishing video swarm failed", zap.Error(errors.New("unknown network")))
					}

					if err := v.PublishSwarm(svc); err != nil {
						s.logger.Debug("publishing video swarm failed", zap.Error(err))
					}
				}()
			}

			go func() {
				<-c.CloseNotify()
				s.logger.Debug("rtmp stream closed", zap.String("key", a.Key))
				v.Stop()
			}()
		},
	}
	go func() {
		if err := rtmp.Listen(); err != nil {
			s.logger.Fatal("rtmp server listen failed", zap.Error(err))
		}
	}()
	return &pb.StartRTMPIngressResponse{}, nil
}
