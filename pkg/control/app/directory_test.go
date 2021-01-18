package app

import (
	"context"
	"testing"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestDirectory(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.Nil(t, err)

	done := make(chan struct{})

	go func() {
		events := ctrl[1].Directory().ReadEvents(context.Background(), networkKey)
		for e := range events {
			if e.GetPublish().GetListing() != nil {
				break
			}
		}

		close(done)
	}()

	publishTicker := time.NewTicker(time.Second)
	for {
		select {
		case <-publishTicker.C:
			key, err := dao.GenerateKey()
			assert.Nil(t, err)

			client, err := ctrl[1].Dialer().Client(networkKey, networkKey, directory.AddressSalt)
			assert.Nil(t, err)

			listing := &networkv1.DirectoryListing{
				Timestamp: time.Now().Unix(),
				Snippet:   &networkv1.DirectoryListingSnippet{},
				Content: &networkv1.DirectoryListing_Media{
					Media: &networkv1.DirectoryListingMedia{
						StartedAt: time.Now().Unix(),
						MimeType:  rtmpingress.TranscoderMimeType,
					},
				},
			}
			err = dao.SignMessage(listing, key)
			assert.Nil(t, err)

			err = networkv1.NewDirectoryClient(client).Publish(
				context.Background(),
				&networkv1.DirectoryPublishRequest{Listing: listing},
				&networkv1.DirectoryPublishResponse{},
			)
			assert.Nil(t, err)
		case <-done:
			return
		}
	}
}
