package transfer

import (
	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
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
	Peer   *vnic.Peer
	Client PeerClient
}

// ID ...
func (p *Peer) ID() kademlia.ID {
	return p.Peer.HostID()
}
