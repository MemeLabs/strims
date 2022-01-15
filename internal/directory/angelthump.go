package directory

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type angelThumpEmbedLoader struct{}

func (t *angelThumpEmbedLoader) BatchSize() int {
	return 1
}

func (t *angelThumpEmbedLoader) Load(ctx context.Context, ids []string) ([]*embedLoaderResult, error) {
	if len(ids) != 1 {
		return nil, errors.New("expected exactly one id")
	}
	res, err := http.Get("https://api.angelthump.com/v3/streams/?username=" + ids[0])
	if err != nil {
		return nil, err
	}

	streams := []struct {
		Type         string `json:"type"`
		ThumbnailURL string `json:"thumbnail_url"`
		ViewerCount  int    `json:"viewer_count"`
		User         struct {
			DisplayName      string `json:"display_name"`
			OfflineBannerURL string `json:"offline_banner_url"`
			ProfileLogoURL   string `json:"profile_logo_url"`
			Title            string `json:"title"`
			Nsfw             bool   `json:"nsfw"`
			Username         string `json:"username"`
		} `json:"user"`
	}{}

	err = json.NewDecoder(res.Body).Decode(&streams)
	if err != nil {
		return nil, err
	}

	var embeds []*embedLoaderResult
	for _, stream := range streams {
		embed := &embedLoaderResult{
			id: stream.User.Username,
			snippet: &networkv1directory.ListingSnippet{
				Live:        stream.Type == "live",
				ViewerCount: uint64(stream.ViewerCount),
				Title:       stream.User.Title,
				IsMature:    stream.User.Nsfw,
				ChannelName: stream.User.DisplayName,
				Thumbnail: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: fmt.Sprintf("%s?_t=%x", stream.ThumbnailURL, timeutil.Now().Unix()),
					},
				},
				ChannelLogo: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: stream.User.ProfileLogoURL,
					},
				},
			},
		}
		if stream.ThumbnailURL == "" {
			embed.snippet.Thumbnail = &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Url{
					Url: stream.User.OfflineBannerURL,
				},
			}
		}

		embeds = append(embeds, embed)
	}
	return embeds, nil
}
