package frontend

import (
	"context"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/control/videochannel"
	"github.com/MemeLabs/go-ppspp/pkg/control/videoingress"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newVideoChannelService(ctx context.Context, logger *zap.Logger, profile *profilev1.Profile, store *dao.ProfileStore, vpnHost *vpn.Host, app *app.Control) videov1.VideoIngressShareService {
	return &videoIngressShareService{
		ctx:     ctx,
		logger:  logger,
		profile: profile,
		store:   store,
		vpnHost: vpnHost,
		app:     app,
	}
}

// videoIngressShareService ...
type videoIngressShareService struct {
	ctx     context.Context
	logger  *zap.Logger
	profile *profilev1.Profile
	store   *dao.ProfileStore
	vpnHost *vpn.Host
	app     *app.Control
}

func (s *videoIngressShareService) toRemoteChannel(ctx context.Context, channel *videov1.VideoChannel) (*videov1.VideoChannel, error) {
	config, err := dao.GetVideoIngressConfig(s.store)
	if err != nil {
		return nil, err
	}

	serverAddr := config.ServerAddr
	if config.PublicServerAddr != "" {
		serverAddr = config.PublicServerAddr
	}

	return &videov1.VideoChannel{
		Owner: &videov1.VideoChannel_RemoteShare_{
			RemoteShare: &videov1.VideoChannel_RemoteShare{
				Id:          channel.Id,
				NetworkKey:  dao.GetRootCert(dialer.VPNCertificate(ctx).GetParent()).Key,
				ServiceKey:  s.profile.Key.Public,
				ServiceSalt: videoingress.ShareAddressSalt,
				ServerAddr:  serverAddr,
			},
		},
		Token:                   channel.Token,
		DirectoryListingSnippet: channel.DirectoryListingSnippet,
	}, nil
}

func (s *videoIngressShareService) CreateChannel(ctx context.Context, r *videov1.VideoIngressShareCreateChannelRequest) (*videov1.VideoIngressShareCreateChannelResponse, error) {
	channel, err := s.app.VideoChannel().CreateChannel(
		videochannel.WithLocalShareOwner(dialer.VPNCertificate(ctx).GetParent()),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}

	remoteChannel, err := s.toRemoteChannel(ctx, channel)
	if err != nil {
		return nil, err
	}

	return &videov1.VideoIngressShareCreateChannelResponse{Channel: remoteChannel}, nil
}

func (s *videoIngressShareService) UpdateChannel(ctx context.Context, r *videov1.VideoIngressShareUpdateChannelRequest) (*videov1.VideoIngressShareUpdateChannelResponse, error) {
	id, err := dao.GetVideoChannelIDByOwnerCert(s.store, dialer.VPNCertificate(ctx).GetParent())
	if err != nil {
		return nil, err
	}

	channel, err := s.app.VideoChannel().UpdateChannel(
		id,
		videochannel.WithLocalShareOwner(dialer.VPNCertificate(ctx).GetParent()),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}

	remoteChannel, err := s.toRemoteChannel(ctx, channel)
	if err != nil {
		return nil, err
	}

	return &videov1.VideoIngressShareUpdateChannelResponse{Channel: remoteChannel}, nil
}

func (s *videoIngressShareService) DeleteChannel(ctx context.Context, r *videov1.VideoIngressShareDeleteChannelRequest) (*videov1.VideoIngressShareDeleteChannelResponse, error) {
	id, err := dao.GetVideoChannelIDByOwnerCert(s.store, dialer.VPNCertificate(ctx).GetParent())
	if err != nil {
		return nil, err
	}

	if err := s.app.VideoChannel().DeleteChannel(id); err != nil {
		return nil, err
	}
	return &videov1.VideoIngressShareDeleteChannelResponse{}, nil
}
