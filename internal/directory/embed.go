package directory

import (
	"context"
	"sync"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/mathutil"
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

type embedService int

const (
	_ embedService = iota
	embedServiceAngelthump
	embedServiceTwitchStream
	embedServiceTwitchVOD
	embedServiceYouTube
	embedServiceSwarm
)

func (s embedService) String() string {
	switch s {
	case embedServiceAngelthump:
		return "Angelthump"
	case embedServiceTwitchStream:
		return "TwitchStream"
	case embedServiceTwitchVOD:
		return "TwitchVOD"
	case embedServiceYouTube:
		return "YouTube"
	case embedServiceSwarm:
		return "Swarm"
	default:
		panic("invalid embed service")
	}
}

func toEmbedService(s networkv1directory.Listing_Embed_Service) (embedService, bool) {
	switch s {
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP:
		return embedServiceAngelthump, true
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM:
		return embedServiceTwitchStream, true
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD:
		return embedServiceTwitchVOD, true
	case networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE:
		return embedServiceYouTube, true
	default:
		return 0, false
	}
}

func newEmbedLoader(logger *zap.Logger, config *networkv1directory.ServerConfig_Integrations) *embedLoader {
	l := &embedLoader{
		logger:  logger,
		loaders: map[embedService]embedServiceLoader{},
	}

	if config.GetAngelthump().GetEnable() {
		l.loaders[embedServiceAngelthump] = &angelThumpEmbedLoader{}
	}

	if config.GetTwitch().GetEnable() {
		api := &twitchAPI{
			ClientID:     config.Twitch.ClientId,
			ClientSecret: config.Twitch.ClientSecret,
		}
		l.loaders[embedServiceTwitchStream] = &twitchStreamEmbedLoader{api}
		l.loaders[embedServiceTwitchVOD] = &twitchVODEmbedLoader{api}
	}

	if config.GetYoutube().GetEnable() {
		l.loaders[embedServiceYouTube] = &youTubeEmbedLoader{
			PublicAPIKey: config.Youtube.PublicApiKey,
		}
	}

	return l
}

type embedLoader struct {
	logger  *zap.Logger
	loaders map[embedService]embedServiceLoader
}

func (l *embedLoader) IsSupported(service embedService) bool {
	_, ok := l.loaders[service]
	return ok
}

func (l *embedLoader) Load(ctx context.Context, sets map[embedService][]string) map[embedService]map[string]*networkv1directory.ListingSnippet {
	var resLock sync.Mutex
	res := map[embedService]map[string]*networkv1directory.ListingSnippet{}
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
			batchIDs := ids[i:mathutil.MinInt(i+n, len(ids))]

			wg.Add(1)
			go func() {
				defer wg.Done()

				embeds, err := loader.Load(ctx, batchIDs)
				if err != nil {
					l.logger.Debug(
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
