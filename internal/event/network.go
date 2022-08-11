// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
)

// NetworkStart ...
type NetworkStart struct {
	Network *networkv1.Network
}

// NetworkStop ...
type NetworkStop struct {
	Network *networkv1.Network
}

// NetworkCertUpdate ...
type NetworkCertUpdate struct {
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

type DirectorySyndicateStart struct {
	Network *networkv1.Network
}

type DirectorySyndicateStop struct {
	Network *networkv1.Network
}
