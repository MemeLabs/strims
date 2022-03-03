package apptest

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/api"
	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/peer"

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
		ioctx := api.NewIOContext(context.Background(), logger, node.VPN, node.Store, &event.Observers{}, nil)
		ctrl[i] = app.NewControl(ioctx, network.NewBroker(logger), node.Profile)

		go ctrl[i].Run()

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
