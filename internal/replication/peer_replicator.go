// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"log"

	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/debug"
)

// NewPeer ...
func newPeerReplicator(
	store dao.Store,
	client *replicationv1.ReplicationPeerClient,
	profile *profilev1.Profile,
) *peerReplicator {
	return &peerReplicator{
		store:   store,
		client:  client,
		profile: profile,

		Checkpoints: newCheckpointMap(),
	}
}

type peerReplicator struct {
	store   dao.Store
	client  *replicationv1.ReplicationPeerClient
	profile *profilev1.Profile

	ReplicaID   uint64
	Checkpoints *checkpointMap
}

func (p *peerReplicator) Open(ctx context.Context, cs *checkpointMap) error {
	req := &replicationv1.PeerOpenRequest{
		StoreVersion: dao.CurrentVersion,
		ReplicaId:    p.profile.DeviceId,
		Checkpoints:  cs.GetAll(),
	}
	var res replicationv1.PeerOpenResponse
	if err := p.client.Open(ctx, req, &res); err != nil {
		return err
	}

	p.ReplicaID = res.ReplicaId
	p.Checkpoints.MergeAll(res.Checkpoints)
	cs.MergeAll(res.Checkpoints)

	if _, err := dao.ReplicationCheckpoints.MergeAll(p.store, res.Checkpoints); err != nil {
		return err
	}
	return nil
}

func (p *peerReplicator) BeginReplication(ctx context.Context, cs []*replicationv1.Checkpoint) error {
	if err := p.AllocateProfileIDs(ctx); err != nil {
		return err
	}

	pc := p.Checkpoints.Get(p.ReplicaID)
	if pc == nil {
		if err := p.beginWithSync(cs, pc); err != nil {
			return err
		}
	} else {
		if err := p.beginWithBootstrap(cs); err != nil {
			return err
		}
	}
	return nil
}

func (p *peerReplicator) beginWithSync(cs []*replicationv1.Checkpoint, pc *replicationv1.Checkpoint) error {
	logs, err := dao.ReplicationEventLogs.GetCompressedDelta(p.store, pc.Version)
	if err != nil {
		return err
	}
	req := &replicationv1.PeerSyncRequest{
		Logs: logs,
	}
	res := &replicationv1.PeerSyncResponse{}
	if err := p.client.Sync(context.Background(), req, res); err != nil {
		return err
	}
	if _, err := dao.ReplicationCheckpoints.Merge(p.store, res.Checkpoint); err != nil {
		return err
	}
	return nil
}

func (p *peerReplicator) beginWithBootstrap(cs []*replicationv1.Checkpoint) error {
	events, err := p.store.Dump()
	if err != nil {
		return err
	}
	logs, err := dao.ReplicationEventLogs.GetAll(p.store)
	if err != nil {
		return err
	}
	req := &replicationv1.PeerBootstrapRequest{
		Events: events,
		Logs:   logs,
	}
	res := &replicationv1.PeerBootstrapResponse{}
	if err := p.client.Bootstrap(context.Background(), req, res); err != nil {
		return err
	}
	if _, err := dao.ReplicationCheckpoints.Merge(p.store, res.Checkpoint); err != nil {
		return err
	}
	return nil
}

func (p *peerReplicator) AllocateProfileIDs(ctx context.Context) error {
	n, err := dao.ProfileID.FreeCount(p.store)
	log.Println(">>>", n, err)
	if err != nil || n > 10000 {
		return err
	}

	log.Println("gib ids")
	var res replicationv1.PeerAllocateProfileIDsResponse
	if err := p.client.AllocateProfileIDs(ctx, &replicationv1.PeerAllocateProfileIDsRequest{}, &res); err != nil {
		return err
	}
	debug.PrintJSON(&res)
	return dao.ProfileID.Push(p.store, res.ProfileId)
}

func (p *peerReplicator) Sync(ctx context.Context, l *replicationv1.EventLog) error {
	req := &replicationv1.PeerSyncRequest{
		Logs: []*replicationv1.EventLog{l},
	}
	res := &replicationv1.PeerSyncResponse{}
	if err := p.client.Sync(context.Background(), req, res); err != nil {
		return err
	}
	if _, err := dao.ReplicationCheckpoints.Merge(p.store, res.Checkpoint); err != nil {
		return err
	}
	return nil
}

func (p *peerReplicator) Close() {
	// do we have anything to do here...?
}
