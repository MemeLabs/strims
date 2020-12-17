// +build js

package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newVideoIngressService(ctx context.Context, logger *zap.Logger, profile *pb.Profile, store *dao.ProfileStore, vpnHost *vpn.Host, app *app.Control) api.VideoIngressService {
	return &videoIngressService{
		ctx:     ctx,
		logger:  logger,
		profile: profile,
		store:   store,
		vpnHost: vpnHost,
		app:     app,
	}
}

// videoIngressService ...
type videoIngressService struct {
	ctx     context.Context
	logger  *zap.Logger
	profile *pb.Profile
	store   *dao.ProfileStore
	vpnHost *vpn.Host
	app     *app.Control
}

func (s *videoIngressService) IsSupported(ctx context.Context, r *pb.VideoIngressIsSupportedRequest) (*pb.VideoIngressIsSupportedResponse, error) {
	return &pb.VideoIngressIsSupportedResponse{Supported: false}, nil
}

func (s *videoIngressService) GetConfig(ctx context.Context, r *pb.VideoIngressGetConfigRequest) (*pb.VideoIngressGetConfigResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) SetConfig(ctx context.Context, r *pb.VideoIngressSetConfigRequest) (*pb.VideoIngressSetConfigResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) ListStreams(ctx context.Context, r *pb.VideoIngressListStreamsRequest) (*pb.VideoIngressListStreamsResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) ListChannels(ctx context.Context, r *pb.VideoIngressListChannelsRequest) (*pb.VideoIngressListChannelsResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) CreateChannel(ctx context.Context, r *pb.VideoIngressCreateChannelRequest) (*pb.VideoIngressCreateChannelResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) UpdateChannel(ctx context.Context, r *pb.VideoIngressUpdateChannelRequest) (*pb.VideoIngressUpdateChannelResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) DeleteChannel(ctx context.Context, r *pb.VideoIngressDeleteChannelRequest) (*pb.VideoIngressDeleteChannelResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *videoIngressService) GetChannelURL(ctx context.Context, r *pb.VideoIngressGetChannelURLRequest) (*pb.VideoIngressGetChannelURLResponse, error) {
	return nil, api.ErrNotImplemented
}
