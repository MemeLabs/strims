package apptest

import (
	"sync"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/vpn"
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
	cluster.Run()

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
