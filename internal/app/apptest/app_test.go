// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package apptest

import (
	"context"
	"time"

	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/peer"
	"github.com/MemeLabs/strims/pkg/httputil"

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
