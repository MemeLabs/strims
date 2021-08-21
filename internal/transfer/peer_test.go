package transfer

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/services/servicestest"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestPeerThing(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	cluster := &servicestest.Cluster{
		Logger:       logger,
		NodeCount:    2,
		PeersPerNode: 1,
	}
	cluster.Run()

	c0 := NewControl(logger, cluster.Hosts[0].VPN, &event.Observers{})
	c1 := NewControl(logger, cluster.Hosts[1].VPN, &event.Observers{})

	_ = c0
	_ = c1
}
