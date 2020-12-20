// +build !js

package frontend

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		api.RegisterVideoIngressService(server, &videoIngressService{
			profile: params.Profile,
			app:     params.App,
		})
	})
}

// videoIngressService ...
type videoIngressService struct {
	profile *pb.Profile
	app     *app.Control
}

func (s *videoIngressService) IsSupported(ctx context.Context, r *pb.VideoIngressIsSupportedRequest) (*pb.VideoIngressIsSupportedResponse, error) {
	return &pb.VideoIngressIsSupportedResponse{Supported: true}, nil
}

func (s *videoIngressService) GetConfig(ctx context.Context, r *pb.VideoIngressGetConfigRequest) (*pb.VideoIngressGetConfigResponse, error) {
	config, err := s.app.VideoIngress().GetIngressConfig()
	if err != nil {
		return nil, err
	}
	return &pb.VideoIngressGetConfigResponse{Config: config}, nil
}

func (s *videoIngressService) SetConfig(ctx context.Context, r *pb.VideoIngressSetConfigRequest) (*pb.VideoIngressSetConfigResponse, error) {
	if err := s.app.VideoIngress().SetIngressConfig(r.Config); err != nil {
		return nil, err
	}
	return &pb.VideoIngressSetConfigResponse{Config: r.Config}, nil
}

func (s *videoIngressService) ListStreams(ctx context.Context, r *pb.VideoIngressListStreamsRequest) (*pb.VideoIngressListStreamsResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) GetChannelURL(ctx context.Context, r *pb.VideoIngressGetChannelURLRequest) (*pb.VideoIngressGetChannelURLResponse, error) {
	channel, err := s.app.VideoChannel().GetChannel(r.Id)
	if err != nil {
		return nil, err
	}

	var serverAddr string
	var id uint64

	switch o := channel.Owner.(type) {
	case *pb.VideoChannel_Local_:
		config, err := s.app.VideoIngress().GetIngressConfig()
		if err != nil {
			return nil, err
		}

		serverAddr = config.ServerAddr
		if config.PublicServerAddr != "" {
			serverAddr = config.PublicServerAddr
		}

		id = channel.Id
	case *pb.VideoChannel_RemoteShare_:
		serverAddr = o.RemoteShare.ServerAddr
		id = o.RemoteShare.Id
	default:
		return nil, errors.New("cannot generate stream key for channel")
	}

	key := dao.FormatVideoChannelStreamKey(id, channel.Token, s.profile.Key)

	return &pb.VideoIngressGetChannelURLResponse{
		Url:        fmt.Sprintf("rtmp://%s/live/%s", serverAddr, key),
		ServerAddr: serverAddr,
		StreamKey:  key,
	}, nil
}
