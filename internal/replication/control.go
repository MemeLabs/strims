// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"bytes"
	"context"
	"log"
	"runtime"
	"strconv"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/dao/versionvector"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/peer"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

const eventLogGCDebounceWait = time.Second

type Control interface {
	peer.PeerHandler
	Run()
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store dao.Store,
	observers *event.Observers,
	profile *profilev1.Profile,
) Control {
	return &control{
		ctx:       ctx,
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
		profile:   profile,
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	vpn       *vpn.Host
	store     dao.Store
	observers *event.Observers
	profile   *profilev1.Profile

	peers syncutil.Map[uint64, *peerService]
}

// AddPeer ...
func (t *control) AddPeer(id uint64, peer *vnic.Peer, s *rpc.Server, c rpc.Caller) {
	if !bytes.Equal(peer.Certificate.Key, t.profile.Key.Public) {
		return
	}

	client := replicationv1.NewReplicationPeerClient(c)
	p := newPeer(id, peer, client, t.logger, t.store, t.profile)
	replicationv1.RegisterReplicationPeerService(s, p)

	t.peers.Set(p.id, p)
}

// RemovePeer ...
func (t *control) RemovePeer(id uint64) {
	t.peers.Delete(id)
}

// Run ...
func (t *control) Run() {
	go t.initDevice()

	for t.ctx.Err() == nil {
		if err := t.tryRunReplicator(); err != nil {
			t.logger.Debug("replicator failed", zap.Error(err))
		}
	}
}

func (t *control) initDevice() {
	// TODO: find somewhere to put this
	dao.Devices.Transform(t.store, t.profile.DeviceId, func(p *profilev1.Device) error {
		p.Os = runtime.GOOS
		p.LastLogin = timeutil.Now().Unix()
		return nil
	})
}

func (t *control) tryRunReplicator() error {
	t.logger.Debug("waiting for replicator lock")
	mu := dao.NewMutex(t.logger, t.store, "replication")
	ctx, err := mu.Lock(t.ctx)
	if err != nil {
		return err
	}
	defer mu.Release()
	return t.runReplicator(ctx)
}

func (t *control) runReplicator(ctx context.Context) error {
	t.logger.Debug("starting replicator")
	r := &replicator{
		ctx:       ctx,
		logger:    t.logger,
		vpn:       t.vpn,
		store:     t.store,
		observers: t.observers,
		profile:   t.profile,

		peers: &t.peers,
	}
	r.gcEventLog = timeutil.DefaultTickEmitter.Debounce(r.runEventLogGC, eventLogGCDebounceWait)
	return r.run()
}

type replicator struct {
	ctx       context.Context
	logger    *zap.Logger
	vpn       *vpn.Host
	store     dao.Store
	observers *event.Observers
	profile   *profilev1.Profile

	checkpoints        *checkpointMap
	gcThreshold        atomic.Pointer[daov1.VersionVector]
	gcEventLog         timeutil.DebouncedFunc
	peers              *syncutil.Map[uint64, *peerService]
	peerReplicators    syncutil.Map[uint64, *peerReplicator]
	peerIDsByReplicaID syncutil.Map[uint64, uint64]

	publisherStopFuncs syncutil.Map[uint64, timeutil.StopFunc]
	scannerStopFuncs   syncutil.Map[uint64, timeutil.StopFunc]

	deviceIDs       syncutil.Set[uint64]
	deviceHostKeyss syncutil.Map[uint64, []byte]

	hack syncutil.Map[kademlia.ID, bool]
}

func (t *replicator) run() error {
	events := make(chan any, 16)
	t.observers.Notify(events)
	defer t.observers.StopNotifying(events)

	ns, err := dao.Networks.GetAll(t.store)
	if err != nil {
		return err
	}
	for _, n := range ns {
		go t.handleNetworkStart(n.Id, dao.NetworkKey(n))
	}

	ds, err := dao.Devices.GetAll(t.store)
	if err != nil {
		return err
	}
	for _, d := range ds {
		t.deviceIDs.Insert(d.Id)
	}

	cs, err := dao.ReplicationCheckpoints.GetAll(t.store)
	if err != nil {
		return err
	}
	t.checkpoints = newCheckpointMap(cs)
	t.gcThreshold.Store(t.checkpoints.MinVersion())

	for _, id := range t.peers.Keys() {
		go t.handlePeerAdd(id)
	}

	go t.scanAllNetworks()

	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case event.PeerAdd:
				go t.handlePeerAdd(e.ID)
			case event.PeerRemove:
				t.handlePeerRemove(e.ID)
			case event.NetworkStart:
				t.handleNetworkStart(e.Network.Id, dao.NetworkKey(e.Network))
			case event.NetworkStop:
				t.handleNetworkStop(e.Network.Id)
			case *profilev1.DeviceChangeEvent:
				t.deviceIDs.Insert(e.Device.Id)
			case *profilev1.DeviceDeleteEvent:
				t.deviceIDs.Delete(e.Device.Id)
			case *replicationv1.CheckpointChangeEvent:
				t.handleCheckpointChange(e.Checkpoint)
			case *replicationv1.EventLog:
				t.handleEventLogChange(e)
			}
		case <-t.ctx.Done():
			return t.ctx.Err()
		}
	}
}

func (t *replicator) handleNetworkStart(id uint64, key []byte) {
	go t.startPublisher(id, key)
	go t.startScanner(id, key)
}

func (t *replicator) handleNetworkStop(id uint64) {
	t.stopPublisher(id)
	t.stopScanner(id)
}

func (t *replicator) handleCheckpointChange(c *replicationv1.Checkpoint) {
	t.checkpoints.Set(c)
	if c.Deleted {
		if peerID, ok := t.peerIDsByReplicaID.GetAndDelete(c.Id); ok {
			t.peerReplicators.Delete(peerID)
		}
	} else {
		t.gcEventLog(t.ctx)
	}
}

func (t *replicator) runEventLogGC(ctx context.Context) {
	prev := t.gcThreshold.Load()
	next := t.checkpoints.MinVersion()
	if d, _ := versionvector.Compare(prev, next); d < 0 {
		t.gcThreshold.Store(next)
		_, err := dao.ReplicationEventLogs.GarbageCollect(t.store, next)
		t.logger.Debug(
			"replication event log gc threshold changed",
			versionvector.LogObject("prev", prev),
			versionvector.LogObject("next", next),
			zap.Error(err),
		)
	}
}

func (t *replicator) handleEventLogChange(l *replicationv1.EventLog) {
	t.peerReplicators.Each(func(k uint64, v *peerReplicator) {
		go v.Sync(t.ctx, []*replicationv1.EventLog{l})
	})
}

func (t *replicator) startPublisher(id uint64, key []byte) {
	logger := t.logger.With(
		zap.Uint64("id", id),
		logutil.ByteHex("key", key),
	)

	n, ok := t.vpn.Node(key)
	if !ok {
		logger.Warn("network not found")
		return
	}

	salt := formatSalt(t.store.ReplicaID())
	hostID := t.vpn.VNIC().ID().Bytes(nil)

	p, err := n.HashTable.Set(t.ctx, t.profile.Key, salt, hostID)
	if err != nil {
		logger.Warn("dht publish failed", zap.Error(err))
		return
	}
	t.publisherStopFuncs.Set(id, p.Close)
}

func (t *replicator) stopPublisher(id uint64) {
	if stop, ok := t.publisherStopFuncs.GetAndDelete(id); ok {
		stop()
	}
}

func (t *replicator) startScanner(id uint64, key []byte) {

}

func (t *replicator) stopScanner(id uint64) {
	if stop, ok := t.scannerStopFuncs.GetAndDelete(id); ok {
		stop()
	}
}

func (t *replicator) scanAllNetworks() error {
	stop := timeutil.DefaultTickEmitter.Subscribe(10*time.Second, func(_ timeutil.Time) {
		ds, err := dao.Devices.GetAll(t.store)
		if err != nil {
			log.Println(err)
			return
		}

		for _, d := range ds {
			// check active replicators
			if d.Id == t.profile.DeviceId {
				continue
			}

			salt := formatSalt(d.Id)
			for _, n := range t.vpn.Nodes() {
				ch, err := n.HashTable.Get(t.ctx, t.profile.Key.Public, salt)
				if err != nil {
					log.Println(err)
					return
				}
				for b := range ch {
					hostID, err := kademlia.UnmarshalID(b)
					if err != nil {
						log.Println(err)
						return
					}
					if t.hack.Has(hostID) {
						continue
					}
					if err := n.PeerExchange.Connect((hostID)); err != nil {
						log.Println(err)
					}
					t.hack.Set(hostID, true)
				}
			}
		}
	}, nil)
	<-t.ctx.Done()
	stop()
	return nil
}

func (t *replicator) handlePeerAdd(peerID uint64) {
	p, ok := t.peers.Get(peerID)
	if !ok {
		return
	}

	r, err := newPeerReplicator(t.ctx, p.logger, t.store, p.client, t.profile)
	if err != nil {
		p.logger.Debug("failed to begin replication", zap.Error(err))
		return
	}

	t.peerReplicators.Set(peerID, r)
	t.peerIDsByReplicaID.Set(r.replicaID, peerID)
}

func (t *replicator) handlePeerRemove(peerID uint64) {
	if p, ok := t.peerReplicators.GetAndDelete(peerID); ok {
		t.peerIDsByReplicaID.Delete(p.replicaID)
	}
}

func formatSalt(replicaID uint64) []byte {
	return strconv.AppendUint([]byte("replication:"), replicaID, 36)
}

var _ zapcore.ObjectMarshaler = checkpointLogObjectMarshaler{}

type checkpointLogObjectMarshaler struct {
	c *replicationv1.Checkpoint
}

func (l checkpointLogObjectMarshaler) MarshalLogObject(e zapcore.ObjectEncoder) error {
	e.AddUint64("id", l.c.Id)
	e.AddObject("version", versionvector.LogObjectMarshaler{Value: l.c.Version})
	return nil
}
