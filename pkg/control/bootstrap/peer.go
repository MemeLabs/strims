package bootstrap

import (
	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
)

// Peer ...
type Peer struct {
	vnic           *vnic.Peer
	client         api.PeerClient
	PublishEnabled bool
}
