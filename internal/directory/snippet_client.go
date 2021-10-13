package directory

import (
	"context"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
)

type snippetClient struct {
	// probably dialer control?
	// our signing key
}

func (c *snippetClient) Subscribe(ctx context.Context, hostID []byte, swarmID ppspp.SwarmID) (*networkv1directory.ListingSnippet, error) {
	// and then we subscribe here?
	// dialer with static host addr resolver from publisher host id and fixed snippet svc port

	return nil, nil
}
