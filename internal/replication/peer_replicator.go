// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"errors"

	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"go.uber.org/zap"
)

func newPeerReplicator(
	ctx context.Context,
	logger *zap.Logger,
	store dao.Store,
	client *replicationv1.ReplicationPeerClient,
	profile *profilev1.Profile,
) (*peerReplicator, error) {
	p := &peerReplicator{
		logger:  logger,
		store:   store,
		client:  client,
		profile: profile,
	}

	req := &replicationv1.PeerOpenRequest{
		StoreVersion: dao.CurrentVersion,
		ReplicaId:    p.profile.DeviceId,
	}
	var res replicationv1.PeerOpenResponse
	if err := p.client.Open(ctx, req, &res); err != nil {
		return p, err
	}
	p.replicaID = res.ReplicaId

	c, err := dao.ReplicationCheckpoints.Get(p.store, res.ReplicaId)
	if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
		return p, err
	}
	if c.GetDeleted() {
		return p, errors.New("cannot sync to deleted replica")
	}

	if err := p.AllocateProfileIDs(ctx); err != nil {
		return p, err
	}

	if res.Checkpoint != nil && len(res.Checkpoint.Version.Value) > 1 {
		if err := p.beginWithSync(ctx, res.Checkpoint); err != nil {
			return p, err
		}
	} else {
		if err := p.beginWithBootstrap(ctx); err != nil {
			return p, err
		}
	}
	return p, nil
}

type peerReplicator struct {
	logger    *zap.Logger
	store     dao.Store
	client    *replicationv1.ReplicationPeerClient
	profile   *profilev1.Profile
	replicaID uint64
}

func (p *peerReplicator) beginWithSync(ctx context.Context, pc *replicationv1.Checkpoint) error {
	logs, err := dao.ReplicationEventLogs.GetCompressedDelta(p.store, pc.Version)
	if err != nil {
		return err
	}
	return p.Sync(ctx, logs)
}

func (p *peerReplicator) beginWithBootstrap(ctx context.Context) error {
	req := &replicationv1.PeerBootstrapRequest{}
	p.store.View(func(tx kv.Tx) (err error) {
		req.Events, err = dao.DumpReplicationEvents(tx)
		if err != nil {
			return err
		}
		req.Logs, err = dao.ReplicationEventLogs.GetAll(tx)
		if err != nil {
			return err
		}
		req.Checkpoints, err = dao.ReplicationCheckpoints.GetAll(tx)
		if err != nil {
			return err
		}
		return nil
	})

	res := &replicationv1.PeerBootstrapResponse{}
	if err := p.client.Bootstrap(ctx, req, res); err != nil {
		return err
	}
	p.logger.Debug(
		"sent replication bootstrap",
		zap.Int("events", len(req.Events)),
		zap.Int("logs", len(req.Logs)),
		zap.Object("checkpoint", checkpointLogObjectMarshaler{res.Checkpoint}),
	)
	return nil
}

func (p *peerReplicator) AllocateProfileIDs(ctx context.Context) error {
	n, err := dao.ProfileID.FreeCount(p.store)
	if err != nil || n > 10000 {
		return err
	}

	var res replicationv1.PeerAllocateProfileIDsResponse
	if err := p.client.AllocateProfileIDs(ctx, &replicationv1.PeerAllocateProfileIDsRequest{}, &res); err != nil {
		return err
	}
	return dao.ProfileID.Push(p.store, res.ProfileId)
}

func (p *peerReplicator) Sync(ctx context.Context, logs []*replicationv1.EventLog) error {
	req := &replicationv1.PeerSyncRequest{Logs: logs}
	res := &replicationv1.PeerSyncResponse{}
	if err := p.client.Sync(ctx, req, res); err != nil {
		return err
	}
	p.logger.Debug(
		"sent replication sync",
		zap.Int("logs", len(logs)),
		zap.Object("checkpoint", checkpointLogObjectMarshaler{res.Checkpoint}),
	)
	return nil
}
