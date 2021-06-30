//go:build !js

package frontend

import (
	"context"
	"errors"
	"fmt"

	profile "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		video.RegisterVideoIngressService(server, &videoIngressService{
			profile: params.Profile,
			app:     params.App,
		})
	})
}

// videoIngressService ...
type videoIngressService struct {
	profile *profile.Profile
	app     control.AppControl
}

func (s *videoIngressService) IsSupported(ctx context.Context, r *video.VideoIngressIsSupportedRequest) (*video.VideoIngressIsSupportedResponse, error) {
	return &video.VideoIngressIsSupportedResponse{Supported: true}, nil
}

func (s *videoIngressService) GetConfig(ctx context.Context, r *video.VideoIngressGetConfigRequest) (*video.VideoIngressGetConfigResponse, error) {
	config, err := s.app.VideoIngress().GetIngressConfig()
	if err != nil {
		return nil, err
	}
	return &video.VideoIngressGetConfigResponse{Config: config}, nil
}

func (s *videoIngressService) SetConfig(ctx context.Context, r *video.VideoIngressSetConfigRequest) (*video.VideoIngressSetConfigResponse, error) {
	if err := s.app.VideoIngress().SetIngressConfig(r.Config); err != nil {
		return nil, err
	}
	return &video.VideoIngressSetConfigResponse{Config: r.Config}, nil
}

func (s *videoIngressService) ListStreams(ctx context.Context, r *video.VideoIngressListStreamsRequest) (*video.VideoIngressListStreamsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *videoIngressService) GetChannelURL(ctx context.Context, r *video.VideoIngressGetChannelURLRequest) (*video.VideoIngressGetChannelURLResponse, error) {
	channel, err := s.app.VideoChannel().GetChannel(r.Id)
	if err != nil {
		return nil, err
	}

	var serverAddr string
	var id uint64

	switch o := channel.Owner.(type) {
	case *video.VideoChannel_Local_:
		config, err := s.app.VideoIngress().GetIngressConfig()
		if err != nil {
			return nil, err
		}

		serverAddr = config.ServerAddr
		if config.PublicServerAddr != "" {
			serverAddr = config.PublicServerAddr
		}

		id = channel.Id
	case *video.VideoChannel_RemoteShare_:
		serverAddr = o.RemoteShare.ServerAddr
		id = o.RemoteShare.Id
	default:
		return nil, errors.New("cannot generate stream key for channel")
	}

	key := dao.FormatVideoChannelStreamKey(id, channel.Token, s.profile.Key)

	return &video.VideoIngressGetChannelURLResponse{
		Url:        fmt.Sprintf("rtmp://%s/live/%s", serverAddr, key),
		ServerAddr: serverAddr,
		StreamKey:  key,
	}, nil
}
