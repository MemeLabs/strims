package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/videochannel"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		api.RegisterVideoChannelService(server, &videoChannelService{
			profile: params.Profile,
			app:     params.App,
		})
	})
}

// videoChannelService ...
type videoChannelService struct {
	profile *pb.Profile
	app     control.AppControl
}

func (s *videoChannelService) List(ctx context.Context, r *pb.VideoChannelListRequest) (*pb.VideoChannelListResponse, error) {
	channels, err := s.app.VideoChannel().ListChannels()
	if err != nil {
		return nil, err
	}
	return &pb.VideoChannelListResponse{Channels: channels}, nil
}

func (s *videoChannelService) Create(ctx context.Context, r *pb.VideoChannelCreateRequest) (*pb.VideoChannelCreateResponse, error) {
	channel, err := s.app.VideoChannel().CreateChannel(
		videochannel.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &pb.VideoChannelCreateResponse{Channel: channel}, nil
}

func (s *videoChannelService) Update(ctx context.Context, r *pb.VideoChannelUpdateRequest) (*pb.VideoChannelUpdateResponse, error) {
	channel, err := s.app.VideoChannel().UpdateChannel(
		r.Id,
		videochannel.WithLocalOwner(s.profile.Key.Public, r.NetworkKey),
		videochannel.WithDirectorySnippet(r.DirectoryListingSnippet),
	)
	if err != nil {
		return nil, err
	}
	return &pb.VideoChannelUpdateResponse{Channel: channel}, nil
}

func (s *videoChannelService) Delete(ctx context.Context, r *pb.VideoChannelDeleteRequest) (*pb.VideoChannelDeleteResponse, error) {
	if err := s.app.VideoChannel().DeleteChannel(r.Id); err != nil {
		return nil, err
	}
	return &pb.VideoChannelDeleteResponse{}, nil
}
