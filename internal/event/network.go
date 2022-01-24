package event

import (
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
)

// NetworkStart ...
type NetworkStart struct {
	Network *networkv1.Network
}

// NetworkStop ...
type NetworkStop struct {
	Network *networkv1.Network
}

// NetworkCertUpdateError ...
type NetworkCertUpdateError struct {
	Network *networkv1.Network
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
	Broadcast  *networkv1directory.EventBroadcast
}

type NetworkBootstrapClientAdd struct {
	Client *networkv1bootstrap.BootstrapClient
}
