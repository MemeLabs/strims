package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/videochannel"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		videov1.RegisterVideoChannelFrontendService(server, &videoChannelService{
			profile: params.Profile,
			app:     params.App,
		})
	})
}

// videoChannelService ...
type videoChannelService struct {
	profile *profilev1.Profile
	app     app.Control
}

func (s *videoChannelService) List(ctx context.Context, r *videov1.VideoChannelListRequest) (*videov1.VideoChannelListResponse, error) {
	channels, err := s.app.VideoChannel().ListChannels()
	if err != nil {
		return nil, err
	}
	return &videov1.VideoChannelListResponse{Channels: channels}, nil
}

func (s *videoChannelService) Create(ctx context.Context, r *videov1.VideoChannelCreateRequest) (*videov1.VideoChannelCreateResponse, error) {
	channel, err := s.app.VideoChannel().CreateChannel(
		videochannel.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &videov1.VideoChannelCreateResponse{Channel: channel}, nil
}

func (s *videoChannelService) Update(ctx context.Context, r *videov1.VideoChannelUpdateRequest) (*videov1.VideoChannelUpdateResponse, error) {
	channel, err := s.app.VideoChannel().UpdateChannel(
		r.Id,
		videochannel.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &videov1.VideoChannelUpdateResponse{Channel: channel}, nil
}

func (s *videoChannelService) Delete(ctx context.Context, r *videov1.VideoChannelDeleteRequest) (*videov1.VideoChannelDeleteResponse, error) {
	if err := s.app.VideoChannel().DeleteChannel(r.Id); err != nil {
		return nil, err
	}
	return &videov1.VideoChannelDeleteResponse{}, nil
}
