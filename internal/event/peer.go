// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import "github.com/MemeLabs/strims/pkg/kademlia"

// PeerAdd ...
type PeerAdd struct {
	ID     uint64
	HostID kademlia.ID
}

// PeerRemove ...
type PeerRemove struct {
	ID     uint64
	HostID kademlia.ID
}
