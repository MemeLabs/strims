package directory

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/set"
	"google.golang.org/api/youtube/v3"
)

type youTubeEmbedLoader struct {
	PublicAPIKey string
}

func (t *youTubeEmbedLoader) getAPIData(path string, data interface{}) error {
	res, err := http.Get(fmt.Sprintf("https://www.googleapis.com/%s&key=%s&maxResults=50", path, t.PublicAPIKey))
	if err != nil {
		return err
	}

	return json.NewDecoder(res.Body).Decode(data)
}

func (t *youTubeEmbedLoader) getVideos(ctx context.Context, ids []string) (*youtube.VideoListResponse, error) {
	data := &youtube.VideoListResponse{}
	err := t.getAPIData("youtube/v3/videos?part=id,liveStreamingDetails,snippet,statistics,contentDetails&id="+strings.Join(ids, ","), data)
	return data, err
}

func (t *youTubeEmbedLoader) getChannels(ctx context.Context, ids []string) (*youtube.ChannelListResponse, error) {
	data := &youtube.ChannelListResponse{}
	err := t.getAPIData("youtube/v3/channels?part=id,snippet&id="+strings.Join(ids, ","), data)
	return data, err
}

func (t *youTubeEmbedLoader) BatchSize() int {
	// see maxResults parameter bounds in youtube v3 videos/channels docs
	// https://developers.google.com/youtube/v3/docs/videos/list
	// https://developers.google.com/youtube/v3/docs/channels/list
	return 50
}

func (t *youTubeEmbedLoader) Load(ctx context.Context, ids []string) ([]*embedLoaderResult, error) {
	videos, err := t.getVideos(ctx, ids)
	if err != nil {
		return nil, err
	}

	channelIDs := set.NewString(len(videos.Items))
	for _, video := range videos.Items {
		channelIDs.Insert(video.Snippet.ChannelId)
	}

	channels, err := t.getChannels(ctx, channelIDs.Slice())
	if err != nil {
		return nil, err
	}

	channelsByID := map[string]*youtube.Channel{}
	for _, channel := range channels.Items {
		channelsByID[channel.Id] = channel
	}

	var embeds []*embedLoaderResult
	for _, video := range videos.Items {
		embed := &embedLoaderResult{
			id: video.Id,
			snippet: &networkv1directory.ListingSnippet{
				Title:    video.Snippet.Title,
				IsMature: video.ContentDetails.ContentRating.YtRating == "ytAgeRestricted",
				Thumbnail: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: video.Snippet.Thumbnails.Medium.Url,
					},
				},
			},
		}

		if channel, ok := channelsByID[video.Snippet.ChannelId]; ok {
			embed.snippet.ChannelName = channel.Snippet.Title
			embed.snippet.ChannelLogo = &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Url{
					Url: channel.Snippet.Thumbnails.Medium.Url,
				},
			}
		}

		if video.LiveStreamingDetails != nil {
			embed.snippet.Live = true
			embed.snippet.ViewerCount = video.LiveStreamingDetails.ConcurrentViewers
		} else {
			embed.snippet.ViewerCount = video.Statistics.ViewCount
		}

		embeds = append(embeds, embed)
	}
	return embeds, nil
}
