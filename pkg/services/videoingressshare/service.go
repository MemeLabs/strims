package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/control/videochannel"
	"github.com/MemeLabs/go-ppspp/pkg/control/videoingress"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newVideoChannelService(ctx context.Context, logger *zap.Logger, profile *pb.Profile, store *dao.ProfileStore, vpnHost *vpn.Host, app *app.Control) api.VideoIngressShareService {
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
	profile *pb.Profile
	store   *dao.ProfileStore
	vpnHost *vpn.Host
	app     *app.Control
}

func (s *videoIngressShareService) toRemoteChannel(ctx context.Context, channel *pb.VideoChannel) (*pb.VideoChannel, error) {
	config, err := dao.GetVideoIngressConfig(s.store)
	if err != nil {
		return nil, err
	}

	serverAddr := config.ServerAddr
	if config.PublicServerAddr != "" {
		serverAddr = config.PublicServerAddr
	}

	return &pb.VideoChannel{
		Owner: &pb.VideoChannel_RemoteShare_{
			RemoteShare: &pb.VideoChannel_RemoteShare{
				Id:          channel.Id,
				NetworkKey:  dao.GetRootCert(rpc.VPNCertificate(ctx).GetParent()).Key,
				ServiceKey:  s.profile.Key.Public,
				ServiceSalt: videoingress.ShareAddressSalt,
				ServerAddr:  serverAddr,
			},
		},
		Token:                   channel.Token,
		DirectoryListingSnippet: channel.DirectoryListingSnippet,
	}, nil
}

func (s *videoIngressShareService) CreateChannel(ctx context.Context, r *pb.VideoIngressShareCreateChannelRequest) (*pb.VideoIngressShareCreateChannelResponse, error) {
	channel, err := s.app.VideoChannel().CreateChannel(
		videochannel.WithLocalShareOwner(rpc.VPNCertificate(ctx).GetParent()),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}

	remoteChannel, err := s.toRemoteChannel(ctx, channel)
	if err != nil {
		return nil, err
	}

	return &pb.VideoIngressShareCreateChannelResponse{Channel: remoteChannel}, nil
}

func (s *videoIngressShareService) UpdateChannel(ctx context.Context, r *pb.VideoIngressShareUpdateChannelRequest) (*pb.VideoIngressShareUpdateChannelResponse, error) {
	id, err := dao.GetVideoChannelIDByOwnerCert(s.store, rpc.VPNCertificate(ctx).GetParent())
	if err != nil {
		return nil, err
	}

	channel, err := s.app.VideoChannel().UpdateChannel(
		id,
		videochannel.WithLocalShareOwner(rpc.VPNCertificate(ctx).GetParent()),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}

	remoteChannel, err := s.toRemoteChannel(ctx, channel)
	if err != nil {
		return nil, err
	}

	return &pb.VideoIngressShareUpdateChannelResponse{Channel: remoteChannel}, nil
}

func (s *videoIngressShareService) DeleteChannel(ctx context.Context, r *pb.VideoIngressShareDeleteChannelRequest) (*pb.VideoIngressShareDeleteChannelResponse, error) {
	id, err := dao.GetVideoChannelIDByOwnerCert(s.store, rpc.VPNCertificate(ctx).GetParent())
	if err != nil {
		return nil, err
	}

	if err := s.app.VideoChannel().DeleteChannel(id); err != nil {
		return nil, err
	}
	return &pb.VideoIngressShareDeleteChannelResponse{}, nil
}
