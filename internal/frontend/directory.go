package frontend

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/event"
	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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
		raw := make(chan interface{}, 8)
		s.app.Events().Notify(raw)

		for {
			select {
			case e := <-raw:
				switch e := e.(type) {
				case event.DirectoryEvent:
					ch <- &network.DirectoryFrontendOpenResponse{
						NetworkId:  e.NetworkID,
						NetworkKey: e.NetworkKey,
						Body: &network.DirectoryFrontendOpenResponse_Broadcast{
							Broadcast: e.Broadcast,
						},
					}
				case event.NetworkStop:
					ch <- &network.DirectoryFrontendOpenResponse{
						NetworkId:  e.Network.Id,
						NetworkKey: e.Network.Key.Public,
						Body:       &network.DirectoryFrontendOpenResponse_Close{},
					}
				}
			case <-ctx.Done():
				s.app.Events().StopNotifying(raw)
				close(raw)
				close(ch)
				return
			}
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
		Timestamp: timeutil.Now().Unix(),
		Snippet: &network.DirectoryListingSnippet{
			Title:       "some title",
			Description: "that test description",
			Tags:        []string{"foo", "bar", "baz"},
		},
		Content: &network.DirectoryListing_Media{
			Media: &network.DirectoryListingMedia{
				StartedAt: timeutil.Now().Unix(),
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
