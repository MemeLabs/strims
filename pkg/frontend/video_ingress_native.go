// +build !js

package frontend

import (
	"context"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/control/videoingress"
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

func (s *videoIngressService) ListChannels(ctx context.Context, r *pb.VideoIngressListChannelsRequest) (*pb.VideoIngressListChannelsResponse, error) {
	channels, err := s.app.VideoIngress().ListChannels()
	if err != nil {
		return nil, err
	}
	return &pb.VideoIngressListChannelsResponse{Channels: channels}, nil
}

func (s *videoIngressService) CreateChannel(ctx context.Context, r *pb.VideoIngressCreateChannelRequest) (*pb.VideoIngressCreateChannelResponse, error) {
	channel, err := s.app.VideoIngress().CreateChannel(
		videoingress.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videoingress.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &pb.VideoIngressCreateChannelResponse{Channel: channel}, nil
}

func (s *videoIngressService) UpdateChannel(ctx context.Context, r *pb.VideoIngressUpdateChannelRequest) (*pb.VideoIngressUpdateChannelResponse, error) {
	channel, err := s.app.VideoIngress().UpdateChannel(
		r.Id,
		videoingress.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videoingress.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &pb.VideoIngressUpdateChannelResponse{Channel: channel}, nil
}

func (s *videoIngressService) DeleteChannel(ctx context.Context, r *pb.VideoIngressDeleteChannelRequest) (*pb.VideoIngressDeleteChannelResponse, error) {
	if err := s.app.VideoIngress().DeleteChannel(r.Id); err != nil {
		return nil, err
	}
	return &pb.VideoIngressDeleteChannelResponse{}, nil
}

func (s *videoIngressService) GetChannelURL(ctx context.Context, r *pb.VideoIngressGetChannelURLRequest) (*pb.VideoIngressGetChannelURLResponse, error) {
	channel, err := s.app.VideoIngress().GetChannel(r.Id)
	if err != nil {
		return nil, err
	}

	var serverAddr string
	var id uint64

	switch o := channel.Owner.(type) {
	case *pb.VideoIngressChannel_Local_:
		config, err := s.app.VideoIngress().GetIngressConfig()
		if err != nil {
			return nil, err
		}

		serverAddr = config.ServerAddr
		if config.PublicServerAddr != "" {
			serverAddr = config.PublicServerAddr
		}

		id = channel.Id
	case *pb.VideoIngressChannel_RemoteShare_:
		serverAddr = o.RemoteShare.ServerAddr
		id = o.RemoteShare.Id
	default:
		return nil, errors.New("cannot generate stream key for channel")
	}

	key := dao.FormatVideoIngressChannelStreamKey(id, channel.Token, s.profile.Key)

	return &pb.VideoIngressGetChannelURLResponse{
		Url:        fmt.Sprintf("rtmp://%s/live/%s", serverAddr, key),
		ServerAddr: serverAddr,
		StreamKey:  key,
	}, nil
}
