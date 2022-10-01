// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"log"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/debug"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"go.uber.org/zap"
)

const profileIDAllocCount = 10000

var _ replicationv1.ReplicationPeerService = (*peerService)(nil)

// NewPeer ...
func newPeer(
	id uint64,
	vnicPeer *vnic.Peer,
	client *replicationv1.ReplicationPeerClient,
	logger *zap.Logger,
	store dao.Store,
	profile *profilev1.Profile,
) *peerService {
	return &peerService{
		id:       id,
		client:   client,
		vnicPeer: vnicPeer,
		logger: logger.With(
			zap.Uint64("id", id),
			logutil.ByteHex("host", vnicPeer.HostID().Bytes(nil)),
		),
		store:   store,
		profile: profile,
	}
}

// Peer ...
type peerService struct {
	id       uint64
	vnicPeer *vnic.Peer
	client   *replicationv1.ReplicationPeerClient
	logger   *zap.Logger
	store    dao.Store
	profile  *profilev1.Profile

	lock sync.Mutex
}

func (p *peerService) Open(ctx context.Context, req *replicationv1.PeerOpenRequest) (*replicationv1.PeerOpenResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (p *peerService) SendEvents(ctx context.Context, req *replicationv1.PeerSendEventsRequest) (*replicationv1.PeerSendEventsResponse, error) {
	if err := p.store.DispatchEvent(req.Events); err != nil {
		return nil, err
	}
	return &replicationv1.PeerSendEventsResponse{}, nil
}

func (p *peerService) AllocateProfileIDs(ctx context.Context, req *replicationv1.PeerAllocateProfileIDsRequest) (*replicationv1.PeerAllocateProfileIDsResponse, error) {
	id, err := dao.ProfileID.Pop(p.store, profileIDAllocCount)
	if err != nil {
		return nil, err
	}
	return &replicationv1.PeerAllocateProfileIDsResponse{ProfileId: id}, nil
}

func (p *peerService) close() {}

func (p *peerService) test() {
	prevProfileIDFreeCount, err := dao.ProfileID.FreeCount(p.store)
	if err != nil {
		p.logger.Debug("error checking ids", zap.Error(err))
	}

	if prevProfileIDFreeCount < 1000 {
		res := &replicationv1.PeerAllocateProfileIDsResponse{}
		err := p.client.AllocateProfileIDs(context.Background(), &replicationv1.PeerAllocateProfileIDsRequest{}, res)
		if err != nil {
			p.logger.Debug("error getting ids", zap.Error(err))
		}
		if err := dao.ProfileID.Push(p.store, res.ProfileId); err != nil {
			p.logger.Debug("error delegating ids", zap.Error(err))
		}
	}
	profileIDFreeCount, err := dao.ProfileID.FreeCount(p.store)
	if err != nil {
		p.logger.Debug("error checking ids", zap.Error(err))
	}
	log.Println(">>>", prevProfileIDFreeCount, profileIDFreeCount)

	// req := &replicationv1.PeerOpenRequest{
	// 	Version:              dao.MinCompatibleVersion,
	// 	MinCompatibleVersion: dao.MinCompatibleVersion,
	// 	ReplicaId:            p.profile.DeviceId,
	// }
	// p.client.Open(context.Background(), req, &replicationv1.PeerOpenResponse{})
	// log.Println("<<< wowee")

	events, err := p.store.Dump()
	if err != nil {
		p.logger.Debug("error dumping memes", zap.Error(err))
	}
	req := &replicationv1.PeerSendEventsRequest{
		Events: events,
	}
	debug.PrintJSON(req)
	err = p.client.SendEvents(context.Background(), req, &replicationv1.PeerSendEventsResponse{})
	if err != nil {
		p.logger.Debug("send failed", zap.Error(err))
	}

	ch := make(chan []*replicationv1.Event)
	p.store.Subscribe(ch)
	for e := range ch {
		req := &replicationv1.PeerSendEventsRequest{
			Events: e,
		}
		debug.PrintJSON(req)
		err = p.client.SendEvents(context.Background(), req, &replicationv1.PeerSendEventsResponse{})
		if err != nil {
			p.logger.Debug("send failed", zap.Error(err))
		}
	}
}
