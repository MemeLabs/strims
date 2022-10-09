// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package apptest

import (
	"sync"
	"testing"

	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestSend(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	cluster := &Cluster{
		Logger:       logger,
		NodeCount:    2,
		PeersPerNode: 1,
	}
	assert.NoError(t, cluster.Run())

	var wg sync.WaitGroup

	cluster.Hosts[0].Node.Network.SetHandler(12000, vpn.MessageHandlerFunc(func(msg *vpn.Message) error {
		assert.Equal(t, []byte("test"), msg.Body)
		wg.Done()
		return nil
	}))

	wg.Add(1)
	cluster.Hosts[1].Node.Network.Send(cluster.Hosts[0].VNIC.ID(), 12000, 12000, []byte("test"))

	wg.Wait()
}
