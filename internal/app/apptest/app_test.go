package apptest

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/peer"
	"github.com/MemeLabs/go-ppspp/pkg/httputil"

	"go.uber.org/zap"
)

// NewTestControlPair ...
func NewTestControlPair(logger *zap.Logger) ([]byte, []app.Control, error) {
	cluster := &Cluster{
		Logger:             logger,
		NodeCount:          2,
		PeersPerNode:       1,
		SkipNetworkBinding: true,
	}
	if err := cluster.Run(); err != nil {
		return nil, nil, err
	}

	ctrl := make([]app.Control, len(cluster.Hosts))
	for i, node := range cluster.Hosts {
		ctrl[i] = app.NewControl(context.Background(), logger, node.VPN, node.Store, &event.Observers{}, httputil.NewMapServeMux(), network.NewBroker(logger), node.Profile)

		qosc := node.VPN.VNIC().QOS().AddClass(1)
		h := peer.NewPeerHandler(logger, ctrl[i], node.Store, qosc)
		for _, p := range node.VNIC.Peers() {
			h(p)
		}
	}

	time.Sleep(100 * time.Millisecond)

	networkKey := dao.NetworkKey(cluster.Hosts[0].Network)

	return networkKey, ctrl, nil
}
