//go:build !js

package frontend

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterVideoIngressService(server, &videoIngressService{
			profile: params.Profile,
			app:     params.App,
			store:   params.Store,
		})
	})
}

// videoIngressService ...
type videoIngressService struct {
	profile *profilev1.Profile
	app     app.Control
	store   *dao.ProfileStore
}

func (s *videoIngressService) IsSupported(ctx context.Context, r *videov1.VideoIngressIsSupportedRequest) (*videov1.VideoIngressIsSupportedResponse, error) {
	return &videov1.VideoIngressIsSupportedResponse{Supported: true}, nil
}

func (s *videoIngressService) GetConfig(ctx context.Context, r *videov1.VideoIngressGetConfigRequest) (*videov1.VideoIngressGetConfigResponse, error) {
	config, err := dao.VideoIngressConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &videov1.VideoIngressGetConfigResponse{Config: config}, nil
}

func (s *videoIngressService) SetConfig(ctx context.Context, r *videov1.VideoIngressSetConfigRequest) (*videov1.VideoIngressSetConfigResponse, error) {
	if err := dao.VideoIngressConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &videov1.VideoIngressSetConfigResponse{Config: r.Config}, nil
}

func (s *videoIngressService) ListStreams(ctx context.Context, r *videov1.VideoIngressListStreamsRequest) (*videov1.VideoIngressListStreamsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *videoIngressService) GetChannelURL(ctx context.Context, r *videov1.VideoIngressGetChannelURLRequest) (*videov1.VideoIngressGetChannelURLResponse, error) {
	channel, err := s.app.VideoChannel().GetChannel(r.Id)
	if err != nil {
		return nil, err
	}

	var serverAddr string
	var id uint64

	switch o := channel.Owner.(type) {
	case *videov1.VideoChannel_Local_:
		config, err := dao.VideoIngressConfig.Get(s.store)
		if err != nil {
			return nil, err
		}

		serverAddr = config.ServerAddr
		if config.PublicServerAddr != "" {
			serverAddr = config.PublicServerAddr
		}

		id = channel.Id
	case *videov1.VideoChannel_RemoteShare_:
		serverAddr = o.RemoteShare.ServerAddr
		id = o.RemoteShare.Id
	default:
		return nil, errors.New("cannot generate stream key for channel")
	}

	key := dao.FormatVideoChannelStreamKey(id, channel.Token, s.profile.Key)

	return &videov1.VideoIngressGetChannelURLResponse{
		Url:        fmt.Sprintf("rtmp://%s/live/%s", serverAddr, key),
		ServerAddr: serverAddr,
		StreamKey:  key,
	}, nil
}
