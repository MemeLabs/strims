package peer

import (
	"context"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/protobuf/pkg/rpc"
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
			bootstrap: networkv1bootstrap.NewPeerServiceClient(c),
			ca:        networkv1ca.NewCAPeerClient(c),
			transfer:  transferv1.NewTransferPeerClient(c),
			network:   networkv1.NewNetworkPeerClient(c),
		})

		s := rpc.NewServer(logger, &rpc.RWFDialer{
			Logger:           logger,
			ReadWriteFlusher: rw1,
		})
		networkv1bootstrap.RegisterPeerServiceService(s, &bootstrapService{p, app, store})
		networkv1ca.RegisterCAPeerService(s, &caService{p, app})
		transferv1.RegisterTransferPeerService(s, &transferService{p, app})
		networkv1.RegisterNetworkPeerService(s, &networkService{p, app})

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
	bootstrap *networkv1bootstrap.PeerServiceClient
	ca        *networkv1ca.CAPeerClient
	transfer  *transferv1.TransferPeerClient
	network   *networkv1.NetworkPeerClient
}

func (c *client) Bootstrap() *networkv1bootstrap.PeerServiceClient { return c.bootstrap }
func (c *client) CA() *networkv1ca.CAPeerClient                    { return c.ca }
func (c *client) Transfer() *transferv1.TransferPeerClient         { return c.transfer }
func (c *client) Network() *networkv1.NetworkPeerClient            { return c.network }
