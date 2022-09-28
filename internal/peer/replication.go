// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package peer

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
)

type replicationService struct {
	Peer app.Peer
	App  app.Control
}

func (s *replicationService) Open(
	ctx context.Context,
	req *replicationv1.PeerOpenRequest,
) (*replicationv1.PeerOpenResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *replicationService) SendEvents(
	ctx context.Context,
	req *replicationv1.PeerSendEventsRequest,
) (<-chan *replicationv1.PeerSendEventsResponse, error) {
	return nil, rpc.ErrNotImplemented
}
