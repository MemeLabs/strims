package event

import "github.com/MemeLabs/go-ppspp/pkg/vnic"

// PeerAdd ...
type PeerAdd struct {
	ID   uint64
	VNIC *vnic.Peer
}

// PeerRemove ...
type PeerRemove struct {
	ID uint64
}
