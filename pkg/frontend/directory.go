package frontend

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		api.RegisterDirectoryFrontendService(server, &directoryService{
			app: params.App,
		})
	})
}

// directoryService ...
type directoryService struct {
	app control.AppControl
}

// Open ...
func (s *directoryService) Open(ctx context.Context, r *pb.DirectoryFrontendOpenRequest) (<-chan *pb.DirectoryFrontendOpenResponse, error) {
	ch := make(chan *pb.DirectoryFrontendOpenResponse, 128)

	go func() {
		events := s.app.Directory().ReadEvents(ctx, r.NetworkKey)
		for e := range events {
			logutil.PrintJSON(e)
			ch <- &pb.DirectoryFrontendOpenResponse{Event: e}
		}
	}()

	return ch, nil
}

// Test ...
func (s *directoryService) Test(ctx context.Context, r *pb.DirectoryFrontendTestRequest) (*pb.DirectoryFrontendTestResponse, error) {
	client, err := s.app.Dialer().Client(r.NetworkKey, r.NetworkKey, directory.AddressSalt)
	if err != nil {
		return nil, err
	}

	key, err := dao.GenerateKey()
	if err != nil {
		return nil, err
	}

	listing := &pb.DirectoryListing{
		// Creator:   creator,
		Timestamp: time.Now().Unix(),
		Snippet: &pb.DirectoryListingSnippet{
			Title:       "some title",
			Description: "that test description",
			Tags:        []string{"foo", "bar", "baz"},
		},
		Content: &pb.DirectoryListing_Media{
			Media: &pb.DirectoryListingMedia{
				StartedAt: time.Now().Unix(),
				MimeType:  rtmpingress.TranscoderMimeType,
				// SwarmUri:  s.swarm.URI().String(),
			},
		},
	}
	err = dao.SignMessage(listing, key)
	if err != nil {
		return nil, err
	}

	err = api.NewDirectoryClient(client).Publish(
		context.Background(),
		&pb.DirectoryPublishRequest{Listing: listing},
		&pb.DirectoryPublishResponse{},
	)
	if err != nil {
		return nil, err
	}

	return &pb.DirectoryFrontendTestResponse{}, nil
}
