// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"time"

	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/set"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/nicklaw5/helix"
	"golang.org/x/sync/errgroup"
)

// see first parameter bounds in helix api docs
// https://dev.twitch.tv/docs/api/reference
const twitchAPIMaxResults = 100

type twitchAPI struct {
	ClientID     string
	ClientSecret string

	lock        sync.Mutex
	accessToken string
	eol         time.Time
}

func (t *twitchAPI) BatchSize() int {
	return twitchAPIMaxResults
}

func (t *twitchAPI) getAccessToken() (string, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	now := time.Now()
	if now.Before(t.eol) {
		return t.accessToken, nil
	}

	res, err := http.Post(fmt.Sprintf("https://id.twitch.tv/oauth2/token?client_id=%s&client_secret=%s&grant_type=client_credentials", t.ClientID, t.ClientSecret), "", nil)
	if err != nil {
		return "", err
	}
	token := struct {
		AccessToken string `json:"access_token"`
		ExpiresIn   int64  `json:"expires_in"`
		TokenType   string `json:"token_type"`
	}{}
	if err := json.NewDecoder(res.Body).Decode(&token); err != nil {
		return "", err
	}

	t.accessToken = token.AccessToken
	t.eol = now.Add(time.Duration(token.ExpiresIn) * time.Second)

	return t.accessToken, nil
}

func (t *twitchAPI) getAPIData(path string, data any) error {
	accessToken, err := t.getAccessToken()
	if err != nil {
		return err
	}

	req, err := http.NewRequest("GET", fmt.Sprintf("https://api.twitch.tv/helix/%s&first=%d", path, twitchAPIMaxResults), nil)
	if err != nil {
		return err
	}
	req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", accessToken))
	req.Header.Add("Client-Id", t.ClientID)

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}

	return json.NewDecoder(res.Body).Decode(data)
}

func (t *twitchAPI) getStreams(ctx context.Context, logins []string) (*helix.ManyStreams, error) {
	data := &helix.ManyStreams{}
	err := t.getAPIData("streams?user_login="+strings.Join(logins, "&user_login="), data)
	return data, err
}

func (t *twitchAPI) getUsers(ctx context.Context, logins []string) (*helix.ManyUsers, error) {
	data := &helix.ManyUsers{}
	err := t.getAPIData("users?login="+strings.Join(logins, "&login="), data)
	return data, err
}

func (t *twitchAPI) getVideos(ctx context.Context, ids []string) (*helix.ManyVideos, error) {
	data := &helix.ManyVideos{}
	err := t.getAPIData("videos?id="+strings.Join(ids, "&id="), data)
	return data, err
}

var twitchVODThumbnailTokens = strings.NewReplacer("%{width}", "640", "%{height}", "360")
var twitchStreamThumbnailTokens = strings.NewReplacer("{width}", "640", "{height}", "360")

type twitchVODEmbedLoader struct {
	*twitchAPI
}

func (t *twitchVODEmbedLoader) Load(ctx context.Context, ids []string) ([]*embedLoaderResult, error) {
	videos, err := t.getVideos(ctx, ids)
	if err != nil {
		return nil, err
	}

	userLogins := set.New[string](len(videos.Videos))
	for _, video := range videos.Videos {
		userLogins.Insert(video.UserLogin)
	}

	users, err := t.getUsers(ctx, userLogins.Slice())
	if err != nil {
		return nil, err
	}

	usersByLogin := map[string]helix.User{}
	for _, user := range users.Users {
		usersByLogin[user.Login] = user
	}

	var embeds []*embedLoaderResult
	for _, video := range videos.Videos {
		embed := &embedLoaderResult{
			id: video.ID,
			snippet: &networkv1directory.ListingSnippet{
				UserCount: uint64(video.ViewCount),
				Title:     video.Title,
				IsMature:  false,
				Thumbnail: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: twitchVODThumbnailTokens.Replace(video.ThumbnailURL),
					},
				},
			},
		}

		if user, ok := usersByLogin[video.UserLogin]; ok {
			embed.snippet.ChannelName = user.DisplayName
			embed.snippet.ChannelLogo = &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Url{
					Url: user.ProfileImageURL,
				},
			}
		}

		embeds = append(embeds, embed)
	}
	return embeds, nil
}

type twitchStreamEmbedLoader struct {
	*twitchAPI
}

func (t *twitchStreamEmbedLoader) Load(ctx context.Context, ids []string) ([]*embedLoaderResult, error) {
	var users *helix.ManyUsers
	var streams *helix.ManyStreams

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() (err error) {
		users, err = t.getUsers(ctx, ids)
		return
	})
	eg.Go(func() (err error) {
		streams, err = t.getStreams(ctx, ids)
		return
	})
	if err := eg.Wait(); err != nil {
		return nil, err
	}

	streamsByUserID := map[string]helix.Stream{}
	for _, stream := range streams.Streams {
		streamsByUserID[stream.UserID] = stream
	}

	var embeds []*embedLoaderResult
	for _, user := range users.Users {
		embed := &embedLoaderResult{
			id: user.Login,
			snippet: &networkv1directory.ListingSnippet{
				ChannelName: user.DisplayName,
				Thumbnail: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: user.OfflineImageURL,
					},
				},
				ChannelLogo: &networkv1directory.ListingSnippetImage{
					SourceOneof: &networkv1directory.ListingSnippetImage_Url{
						Url: user.ProfileImageURL,
					},
				},
			},
		}

		if stream, ok := streamsByUserID[user.ID]; ok {
			embed.snippet.Live = true
			embed.snippet.UserCount = uint64(stream.ViewerCount)
			embed.snippet.Title = stream.Title
			embed.snippet.Thumbnail = &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Url{
					Url: fmt.Sprintf("%s?_t=%x", twitchStreamThumbnailTokens.Replace(stream.ThumbnailURL), timeutil.Now().Unix()),
				},
			}
			embed.snippet.IsMature = stream.IsMature
		}

		embeds = append(embeds, embed)
	}
	return embeds, nil
}
