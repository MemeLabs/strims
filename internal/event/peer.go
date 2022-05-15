// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

// PeerAdd ...
type PeerAdd struct {
	ID uint64
}

// PeerRemove ...
type PeerRemove struct {
	ID uint64
}
