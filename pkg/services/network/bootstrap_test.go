package network

import (
	"context"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/go-ppspp/pkg/services/servicestest"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestBootstrap(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	n0, err := servicestest.NewNode(logger, 0)
	assert.Nil(t, err)
	n1, err := servicestest.NewNode(logger, 0)
	assert.Nil(t, err)

	// ---

	net0, err := dao.NewNetwork("network", nil, n0.Profile)
	assert.Nil(t, err)

	invitation, err := dao.NewInvitationV0(n0.Profile.Key, net0.Certificate)
	assert.Nil(t, err)

	net1, err := dao.NewNetworkFromInvitationV0(invitation, n1.Profile)
	assert.Nil(t, err)

	client0, err := n0.VPN.AddNetwork(net0.Certificate)
	assert.Nil(t, err)
	_, err = n1.VPN.AddNetwork(net1.Certificate)
	assert.Nil(t, err)

	ctx := context.Background()

	_, err = NewCA(ctx, logger, client0, net0)
	assert.Nil(t, err)

	time.Sleep(time.Second)
	// ---

	NewPeerHandler(logger, NewBroker(logger), n0.VPN, n0.Profile)
	NewPeerHandler(logger, NewBroker(logger), n1.VPN, n1.Profile)

	c0, c1 := ppspptest.NewUnbufferedConnPair()

	n0.VNIC.AddLink(c0)
	n1.VNIC.AddLink(c1)

	// ---

	time.Sleep(10 * time.Second)
}
