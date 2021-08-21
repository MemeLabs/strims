package event

import "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"

// NetworkLoad ...
type NetworkLoad struct {
	Network *network.Network
}

// NetworkAdd ...
type NetworkAdd struct {
	Network *network.Network
}

// NetworkRemove ...
type NetworkRemove struct {
	Network *network.Network
}

// NetworkStart ...
type NetworkStart struct {
	Network *network.Network
}

// NetworkStop ...
type NetworkStop struct {
	Network *network.Network
}

// NetworkCertUpdate ...
type NetworkCertUpdate struct {
	Network *network.Network
}

// NetworkCertUpdateError ...
type NetworkCertUpdateError struct {
	Network *network.Network
	Error   error
}

// NetworkPeerBindings ...
type NetworkPeerBindings struct {
	PeerID      uint64
	NetworkKeys [][]byte
}

// NetworkPeerOpen ...
type NetworkPeerOpen struct {
	PeerID     uint64
	NetworkID  uint64
	NetworkKey []byte
}

// NetworkPeerClose ...
type NetworkPeerClose struct {
	PeerID     uint64
	NetworkID  uint64
	NetworkKey []byte
}

// NetworkPeerCountUpdate ...
type NetworkPeerCountUpdate struct {
	NetworkID uint64
	PeerCount int
}

// NetworkNegotiationComplete ...
type NetworkNegotiationComplete struct{}

type DirectoryEvent struct {
	NetworkID  uint64
	NetworkKey []byte
	Broadcast  *network.DirectoryEventBroadcast
}
