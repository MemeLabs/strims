package directory

import (
	"context"
	"log"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/image"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestFoo(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	logger, err := zap.NewDevelopment()
	assert.NoError(t, err)

	a, b := ppspptest.NewConnPair()
	rpcServer := rpc.NewServer(logger, &rpc.RWFDialer{
		Logger:           logger,
		ReadWriteFlusher: a,
	})

	key, err := dao.GenerateKey()
	assert.NoError(t, err)
	swarmID := ppspp.NewSwarmID(key.Public)

	service := &snippetService{}
	networkv1directory.RegisterDirectorySnippetService(rpcServer, service)

	go rpcServer.Listen(ctx)

	go func() {
		snippet := &networkv1directory.ListingSnippet{
			Title:       "title",
			Description: "description",
			Tags:        []string{"a", "b", "c"},
			Category:    "category",
			ChannelName: "channel name",
			ViewerCount: 100,
			Live:        true,
			IsMature:    true,
			Thumbnail: &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Image{
					Image: &image.Image{
						Type: image.ImageType_IMAGE_TYPE_UNDEFINED,
						Data: make([]byte, 128),
					},
				},
			},
			ChannelLogo: &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Image{
					Image: &image.Image{
						Type: image.ImageType_IMAGE_TYPE_UNDEFINED,
						Data: make([]byte, 128),
					},
				},
			},
		}
		err := dao.SignMessage(snippet, key)
		assert.NoError(t, err)

		log.Println("calling UpdateSnippet")
		service.UpdateSnippet(swarmID, snippet)
	}()

	time.Sleep(time.Millisecond)

	rpcClient, err := rpc.NewClient(logger, &rpc.RWFDialer{
		Logger:           logger,
		ReadWriteFlusher: b,
	})
	assert.NoError(t, err)
	client := networkv1directory.NewDirectorySnippetClient(rpcClient)

	req := &networkv1directory.SnippetSubscribeRequest{SwarmId: swarmID}
	res := make(chan *networkv1directory.SnippetSubscribeResponse, 16)

	go func() {
		var n int
		for delta := range res {
			debug.PrintJSON(delta)
			if n++; n == 2 {
				cancel()
			}
		}
	}()

	log.Println("calling Subscribe")
	err = client.Subscribe(ctx, req, res)
	assert.ErrorIs(t, err, context.Canceled)
}
