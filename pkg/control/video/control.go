package video

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, observers *event.Observers) *Control {
	events := make(chan interface{}, 128)
	observers.Peer.Notify(events)
	observers.Network.Notify(events)

	return &Control{
		logger: logger,
		vpn:    vpn,
		events: events,
	}
}

// Control ...
type Control struct {
	logger *zap.Logger
	vpn    *vpn.Host
	events chan interface{}
}

// Outboundthing ...
func (c *Control) Outboundthing(networkKey []byte) error {
	// client, ok := c.vpn.Client(networkKey)
	// if !ok {
	// 	return errors.New("network not found")
	// }

	// client.PeerIndex.Publish(context.Background(), key []byte, salt []byte, port uint16)

	// svc.Swarms.OpenSwarm(c.s)

	// newSwarmPeerManager(c.ctx, svc, getPeersGetter(c.ctx, svc, c.key, videoSalt))

	// if err := svc.PeerIndex.Publish(c.ctx, c.key, videoSalt, 0); err != nil {
	// 	return err
	// }

	// listing := &pb.DirectoryListing{
	// 	MimeType: "video/webm",
	// 	Title:    "test",
	// 	Key:      c.key,
	// }
	// if err := svc.Directory.Publish(c.ctx, listing); err != nil {
	// 	return err
	// }
	// c.logger.Info("published video swarm", logutil.ByteHex("key", c.key))

	// c.svc = append(c.svc, svc)

	return nil
}

// Video ...
type Video struct {
	ctx      context.Context
	cancel   context.CancelFunc
	key      *pb.Key
	salt     []byte
	mimeType string
	title    string
	swarm    *ppspp.Swarm
}
