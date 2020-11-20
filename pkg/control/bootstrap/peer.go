package bootstrap

import (
	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
)

// PeerClient ..
type PeerClient interface {
	Swarm() *api.SwarmPeerClient
	Bootstrap() *api.BootstrapPeerClient
	CA() *api.CAPeerClient
	Network() *api.NetworkPeerClient
}

// Peer ...
type Peer struct {
	vnic           *vnic.Peer
	client         PeerClient
	PublishEnabled bool
}
