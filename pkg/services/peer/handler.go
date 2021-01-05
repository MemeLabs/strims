package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"go.uber.org/zap"
)

// NewPeerHandler ...
func NewPeerHandler(logger *zap.Logger, app control.AppControl, store *dao.ProfileStore) vnic.PeerHandler {
	return func(peer *vnic.Peer) {
		rw0, rw1 := peer.ChannelPair(vnic.PeerRPCClientPort, vnic.PeerRPCServerPort)

		c, err := rpc.NewClient(logger, &rpc.RWFDialer{
			Logger:           logger,
			ReadWriteFlusher: rw0,
		})
		if err != nil {
			return
		}

		p := app.Peer().Add(peer, &client{
			client:    c,
			bootstrap: api.NewBootstrapPeerClient(c),
			ca:        api.NewCAPeerClient(c),
			transfer:  api.NewTransferPeerClient(c),
			network:   api.NewNetworkPeerClient(c),
		})

		s := rpc.NewServer(logger, &rpc.RWFDialer{
			Logger:           logger,
			ReadWriteFlusher: rw1,
		})
		api.RegisterBootstrapPeerService(s, &bootstrapService{p, app, store})
		api.RegisterCAPeerService(s, &caService{p, app})
		api.RegisterTransferPeerService(s, &transferService{p, app})
		api.RegisterNetworkPeerService(s, &networkService{p, app})

		go func() {
			err := s.Listen(context.Background())
			if err != nil {
				logger.Debug("peer rpc server closed with error", zap.Error(err))
			}
			app.Peer().Remove(p)
			c.Close()
		}()
	}
}

type client struct {
	client    *rpc.Client
	bootstrap *api.BootstrapPeerClient
	ca        *api.CAPeerClient
	transfer  *api.TransferPeerClient
	network   *api.NetworkPeerClient
}

func (c *client) Bootstrap() *api.BootstrapPeerClient { return c.bootstrap }
func (c *client) CA() *api.CAPeerClient               { return c.ca }
func (c *client) Transfer() *api.TransferPeerClient   { return c.transfer }
func (c *client) Network() *api.NetworkPeerClient     { return c.network }
