package event

import "github.com/MemeLabs/go-ppspp/pkg/pb"

// NetworkLoad ...
type NetworkLoad struct {
	Network *pb.Network
}

// NetworkAdd ...
type NetworkAdd struct {
	Network *pb.Network
}

// NetworkRemove ...
type NetworkRemove struct {
	Network *pb.Network
}

// NetworkStart ...
type NetworkStart struct {
	Network *pb.Network
}

// NetworkStop ...
type NetworkStop struct {
	Network *pb.Network
}

// NetworkCertUpdate ...
type NetworkCertUpdate struct {
	Network *pb.Network
}

// NetworkCertUpdateError ...
type NetworkCertUpdateError struct {
	Network *pb.Network
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

// NetworkNegotiationComplete ...
type NetworkNegotiationComplete struct{}
