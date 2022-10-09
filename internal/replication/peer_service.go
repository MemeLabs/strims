// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/dao/versionvector"
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
	pc, err := dao.ReplicationCheckpoints.Get(p.store, req.ReplicaId)
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, err
	}
	if pc.GetDeleted() {
		return nil, errors.New("cannot sync to deleted replica")
	}

	c, err := dao.ReplicationCheckpoints.Get(p.store, p.profile.DeviceId)
	if err != nil {
		return nil, err
	}

	return &replicationv1.PeerOpenResponse{
		StoreVersion: dao.CurrentVersion,
		ReplicaId:    p.profile.DeviceId,
		Checkpoint:   c,
	}, nil
}

func (p *peerService) Bootstrap(ctx context.Context, req *replicationv1.PeerBootstrapRequest) (*replicationv1.PeerBootstrapResponse, error) {
	err := p.store.Update(func(tx kv.RWTx) (err error) {
		if _, err := dao.ReplicationCheckpoints.MergeAll(tx, req.Checkpoints); err != nil {
			return err
		}

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

	v := versionvector.New()
	for _, c := range req.Checkpoints {
		versionvector.Upgrade(v, c.Version)
	}

	c, err := dao.ApplyReplicationEvents(p.store, req.Events, v)
	if err != nil {
		return nil, err
	}

	p.logger.Debug(
		"received replication bootstrap",
		zap.Int("events", len(req.Events)),
		zap.Int("logs", len(req.Logs)),
		zap.Object("checkpoint", checkpointLogObjectMarshaler{c}),
	)
	return &replicationv1.PeerBootstrapResponse{Checkpoint: c}, nil
}

func (p *peerService) Sync(ctx context.Context, req *replicationv1.PeerSyncRequest) (*replicationv1.PeerSyncResponse, error) {
	c, err := dao.ApplyReplicationEventLogs(p.store, req.Logs)
	if err != nil {
		return nil, err
	}
	p.logger.Debug(
		"received replication sync",
		zap.Int("logs", len(req.Logs)),
		zap.Object("checkpoint", checkpointLogObjectMarshaler{c}),
	)
	return &replicationv1.PeerSyncResponse{Checkpoint: c}, nil
}

func (p *peerService) AllocateProfileIDs(ctx context.Context, req *replicationv1.PeerAllocateProfileIDsRequest) (*replicationv1.PeerAllocateProfileIDsResponse, error) {
	id, err := dao.ProfileID.Pop(p.store, profileIDAllocCount)
	if err != nil {
		return nil, err
	}
	return &replicationv1.PeerAllocateProfileIDsResponse{ProfileId: id}, nil
}
