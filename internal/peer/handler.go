package peer

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/rpcutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

const (
	RPCClientRetries = 3
	RPCClientBackoff = 2
	RPCClientDelay   = 100 * time.Millisecond
	RPCClientTimeout = time.Second
)

// NewPeerHandler ...
func NewPeerHandler(logger *zap.Logger, app app.Control, store *dao.ProfileStore, qosc *qos.Class) vnic.PeerHandler {
	return func(peer *vnic.Peer) {
		logger := logger.With(zap.Stringer("host", peer.HostID()))
		rw0, rw1 := peer.ChannelPair(vnic.PeerRPCClientPort, vnic.PeerRPCServerPort, qosc)

		c, err := rpc.NewClient(logger, &rpc.RWFDialer{
			Logger:           logger,
			ReadWriteFlusher: rw0,
		})
		if err != nil {
			logger.Info("creating peer rpc client failed", zap.Error(err))
			return
		}

		rc := rpc.Caller(rpcutil.NewClientRetrier(c, RPCClientRetries, RPCClientBackoff, RPCClientDelay, RPCClientTimeout))
		if logger.Core().Enabled(zap.DebugLevel) {
			rc = rpcutil.NewClientLogger(rc, logger)
		}

		p := app.Peer().Add(peer, &client{
			bootstrap: networkv1bootstrap.NewPeerServiceClient(rc),
			ca:        networkv1ca.NewCAPeerClient(rc),
			transfer:  transferv1.NewTransferPeerClient(rc),
			network:   networkv1.NewNetworkPeerClient(rc),
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
			logger.Debug("peer rpc server listening")
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
	bootstrap *networkv1bootstrap.PeerServiceClient
	ca        *networkv1ca.CAPeerClient
	transfer  *transferv1.TransferPeerClient
	network   *networkv1.NetworkPeerClient
}

func (c *client) Bootstrap() *networkv1bootstrap.PeerServiceClient { return c.bootstrap }
func (c *client) CA() *networkv1ca.CAPeerClient                    { return c.ca }
func (c *client) Transfer() *transferv1.TransferPeerClient         { return c.transfer }
func (c *client) Network() *networkv1.NetworkPeerClient            { return c.network }
