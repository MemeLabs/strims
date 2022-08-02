// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bootstrap

import (
	"sync/atomic"

	"github.com/MemeLabs/strims/internal/api"
	"github.com/MemeLabs/strims/pkg/vnic"
)

type Peer any

var _ Peer = (*peer)(nil)

// Peer ...
type peer struct {
	vnicPeer       *vnic.Peer
	client         api.PeerClient
	PublishEnabled atomic.Bool
}
