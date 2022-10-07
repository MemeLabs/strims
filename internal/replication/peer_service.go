// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kv"
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
	if _, err := dao.ReplicationCheckpoints.MergeAll(p.store, req.Checkpoints); err != nil {
		return nil, err
	}

	c, err := dao.ReplicationCheckpoints.GetAll(p.store)
	if err != nil {
		return nil, err
	}

	return &replicationv1.PeerOpenResponse{
		StoreVersion: dao.CurrentVersion,
		ReplicaId:    p.profile.DeviceId,
		Checkpoints:  c,
	}, nil
}

func (p *peerService) Bootstrap(ctx context.Context, req *replicationv1.PeerBootstrapRequest) (*replicationv1.PeerBootstrapResponse, error) {
	c, err := dao.ApplyReplicationEvents(p.store, req.Events, dao.NewVersionVectorFromReplicationEventLogs(req.Logs))
	if err != nil {
		return nil, err
	}

	err = p.store.Update(func(tx kv.RWTx) (err error) {
		for _, l := range req.Logs {
			if err := dao.ReplicationEventLogs.Insert(tx, l); err != nil {
				return err
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &replicationv1.PeerBootstrapResponse{Checkpoint: c}, nil
}

func (p *peerService) Sync(ctx context.Context, req *replicationv1.PeerSyncRequest) (*replicationv1.PeerSyncResponse, error) {
	c, err := dao.ApplyReplicationEventLogs(p.store, req.Logs)
	if err != nil {
		return nil, err
	}
	return &replicationv1.PeerSyncResponse{Checkpoint: c}, nil
}

func (p *peerService) AllocateProfileIDs(ctx context.Context, req *replicationv1.PeerAllocateProfileIDsRequest) (*replicationv1.PeerAllocateProfileIDsResponse, error) {
	id, err := dao.ProfileID.Pop(p.store, profileIDAllocCount)
	if err != nil {
		return nil, err
	}
	return &replicationv1.PeerAllocateProfileIDsResponse{ProfileId: id}, nil
}

func (p *peerService) close() {}
