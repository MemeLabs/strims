// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package directory

import (
	"context"
	"sync"

	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/mathutil"
	"go.uber.org/zap"
)

type embedLoaderResult struct {
	id      string
	snippet *networkv1directory.ListingSnippet
}

type embedServiceLoader interface {
	BatchSize() int
	Load(ctx context.Context, ids []string) ([]*embedLoaderResult, error)
}

func newEmbedLoader(logger *zap.Logger, config *networkv1directory.ServerConfig_Integrations) *embedLoader {
	l := &embedLoader{
		logger:  logger,
		loaders: map[networkv1directory.Listing_Embed_Service]embedServiceLoader{},
	}

	if config.GetAngelthump().GetEnable() {
		l.loaders[networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP] = &angelThumpEmbedLoader{}
	}

	if config.GetTwitch().GetEnable() {
		api := &twitchAPI{
			ClientID:     config.Twitch.ClientId,
			ClientSecret: config.Twitch.ClientSecret,
		}
		l.loaders[networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM] = &twitchStreamEmbedLoader{api}
		l.loaders[networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD] = &twitchVODEmbedLoader{api}
	}

	if config.GetYoutube().GetEnable() {
		l.loaders[networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE] = &youTubeEmbedLoader{
			PublicAPIKey: config.Youtube.PublicApiKey,
		}
	}

	return l
}

type embedLoader struct {
	logger  *zap.Logger
	loaders map[networkv1directory.Listing_Embed_Service]embedServiceLoader
}

func (l *embedLoader) IsSupported(service networkv1directory.Listing_Embed_Service) bool {
	_, ok := l.loaders[service]
	return ok
}

func (l *embedLoader) Load(ctx context.Context, sets map[networkv1directory.Listing_Embed_Service][]string) map[networkv1directory.Listing_Embed_Service]map[string]*networkv1directory.ListingSnippet {
	var resLock sync.Mutex
	res := map[networkv1directory.Listing_Embed_Service]map[string]*networkv1directory.ListingSnippet{}
	for service := range sets {
		res[service] = map[string]*networkv1directory.ListingSnippet{}
	}

	var wg sync.WaitGroup
	for service, ids := range sets {
		service := service
		loader, ok := l.loaders[service]
		if !ok {
			continue
		}

		n := loader.BatchSize()
		for i := 0; i < len(ids); i += n {
			batchIDs := ids[i:mathutil.Min(i+n, len(ids))]

			wg.Add(1)
			go func() {
				defer wg.Done()

				embeds, err := loader.Load(ctx, batchIDs)
				if err != nil {
					l.logger.Error(
						"directory embed loader failed",
						zap.Stringer("service", service),
						zap.Error(err),
					)
					return
				}

				resLock.Lock()
				defer resLock.Unlock()
				for _, e := range embeds {
					res[service][e.id] = e.snippet
				}
			}()
		}
	}
	wg.Wait()

	return res
}
