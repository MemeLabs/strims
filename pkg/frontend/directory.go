package frontend

import (
	"context"
	"time"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		network.RegisterDirectoryFrontendService(server, &directoryService{
			app: params.App,
		})
	})
}

// directoryService ...
type directoryService struct {
	app control.AppControl
}

// Open ...
func (s *directoryService) Open(ctx context.Context, r *network.DirectoryFrontendOpenRequest) (<-chan *network.DirectoryFrontendOpenResponse, error) {
	ch := make(chan *network.DirectoryFrontendOpenResponse, 128)

	go func() {
		events := s.app.Directory().ReadEvents(ctx, r.NetworkKey)
		for e := range events {
			logutil.PrintJSON(e)
			ch <- &network.DirectoryFrontendOpenResponse{Event: e}
		}
	}()

	return ch, nil
}

// Test ...
func (s *directoryService) Test(ctx context.Context, r *network.DirectoryFrontendTestRequest) (*network.DirectoryFrontendTestResponse, error) {
	client, err := s.app.Dialer().Client(r.NetworkKey, r.NetworkKey, directory.AddressSalt)
	if err != nil {
		return nil, err
	}

	key, err := dao.GenerateKey()
	if err != nil {
		return nil, err
	}

	listing := &network.DirectoryListing{
		// Creator:   creator,
		Timestamp: time.Now().Unix(),
		Snippet: &network.DirectoryListingSnippet{
			Title:       "some title",
			Description: "that test description",
			Tags:        []string{"foo", "bar", "baz"},
		},
		Content: &network.DirectoryListing_Media{
			Media: &network.DirectoryListingMedia{
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

	err = network.NewDirectoryClient(client).Publish(
		context.Background(),
		&network.DirectoryPublishRequest{Listing: listing},
		&network.DirectoryPublishResponse{},
	)
	if err != nil {
		return nil, err
	}

	return &network.DirectoryFrontendTestResponse{}, nil
}
