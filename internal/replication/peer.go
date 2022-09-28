// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"log"
	"sync"

	"github.com/MemeLabs/strims/internal/api"
	"github.com/MemeLabs/strims/internal/event"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

type Peer interface {
	// HandlePeerClose(networkKey []byte)
	// HandlePeerUpdateCertificate(cert *certificate.Certificate) error
}

var _ Peer = (*peer)(nil)

// NewPeer ...
func newPeer(
	id uint64,
	vnicPeer *vnic.Peer,
	client api.PeerClient,
	logger *zap.Logger,
	observers *event.Observers,
	vpn *vpn.Host,
	qosc *qos.Class,
) *peer {
	return &peer{
		id:       id,
		client:   client,
		vnicPeer: vnicPeer,
		logger: logger.With(
			zap.Uint64("id", id),
			logutil.ByteHex("host", vnicPeer.HostID().Bytes(nil)),
		),
		observers: observers,
		vpn:       vpn,

		brokerConn: vnicPeer.Channel(vnic.ReplicationPort, qosc),
	}
}

// Peer ...
type peer struct {
	id        uint64
	vnicPeer  *vnic.Peer
	client    api.PeerClient
	logger    *zap.Logger
	observers *event.Observers
	vpn       *vpn.Host

	lock       sync.Mutex
	brokerConn *vnic.FrameReadWriter
}

func (p *peer) close() {}

func (p *peer) test() {
	p.client.Replication().Open(context.Background(), &replicationv1.PeerOpenRequest{}, &replicationv1.PeerOpenResponse{})
	log.Println("<<< wowee")
}
