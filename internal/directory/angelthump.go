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
	res, err := http.Get("https://api.angelthump.com/v2/streams/" + ids[0])
	if err != nil {
		return nil, err
	}

	data := struct {
		Username     string `json:"username"`
		Type         string `json:"type"`
		ThumbnailURL string `json:"thumbnail_url"`
		ViewerCount  int    `json:"viewer_count"`
		User         struct {
			DisplayName      string `json:"display_name"`
			OfflineBannerURL string `json:"offline_banner_url"`
			ProfileLogoURL   string `json:"profile_logo_url"`
			Title            string `json:"title"`
			Nsfw             bool   `json:"nsfw"`
		} `json:"user"`
	}{}

	err = json.NewDecoder(res.Body).Decode(&data)
	if err != nil {
		return nil, err
	}

	embed := &embedLoaderResult{
		id: data.Username,
		snippet: &networkv1directory.ListingSnippet{
			Live:        data.Type == "live",
			ViewerCount: uint64(data.ViewerCount),
			Title:       data.User.Title,
			IsMature:    data.User.Nsfw,
			ChannelName: data.User.DisplayName,
			Thumbnail: &networkv1directory.ListingSnippet_Image{
				SourceOneof: &networkv1directory.ListingSnippet_Image_Url{
					Url: fmt.Sprintf("%s?_t=%x", data.ThumbnailURL, timeutil.Now().Unix()),
				},
			},
			ChannelLogo: &networkv1directory.ListingSnippet_Image{
				SourceOneof: &networkv1directory.ListingSnippet_Image_Url{
					Url: data.User.ProfileLogoURL,
				},
			},
		},
	}
	if data.ThumbnailURL == "" {
		embed.snippet.Thumbnail = &networkv1directory.ListingSnippet_Image{
			SourceOneof: &networkv1directory.ListingSnippet_Image_Url{
				Url: data.User.OfflineBannerURL,
			},
		}
	}
	return []*embedLoaderResult{embed}, nil
}
