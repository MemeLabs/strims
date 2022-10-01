// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"bytes"
	"context"
	"log"
	"strconv"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/notification"
	"github.com/MemeLabs/strims/internal/peer"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

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
	notification notification.Control,
) Control {
	return &control{
		ctx:          ctx,
		logger:       logger,
		vpn:          vpn,
		store:        store,
		observers:    observers,
		profile:      profile,
		notification: notification,

		events: observers.Chan(),
		qosc:   vpn.VNIC().QOS().AddClass(1),
	}
}

// Control ...
type control struct {
	ctx          context.Context
	logger       *zap.Logger
	vpn          *vpn.Host
	store        dao.Store
	observers    *event.Observers
	profile      *profilev1.Profile
	notification notification.Control

	events chan any
	qosc   *qos.Class
	peers  syncutil.Map[uint64, *peerService]
	memes  syncutil.Map[kademlia.ID, struct{}]
}

// Run ...
func (t *control) Run() {
	go func() {
		for {
			if err := t.doFoo(); err != nil {
				log.Println(err)
			}
			time.Sleep(10 * time.Second)
		}
	}()
	// publish that we're available to the dht
	// search for announcements and connect to replication candidates

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				go t.handlePeerAdd(e.ID)
			case event.PeerRemove:
				t.memes.Delete(e.HostID)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func formatSalt(replicaID uint64) []byte {
	return strconv.AppendUint([]byte("replication:"), replicaID, 10)
}

func (t *control) doFoo() error {
	mu := dao.NewMutex(t.logger, t.store, "replication")
	ctx, err := mu.Lock(t.ctx)
	if err != nil {
		return err
	}
	defer mu.Release()

	eg, ctx := errgroup.WithContext(ctx)
	eg.Go(func() error {
		salt := formatSalt(t.store.ReplicaID())
		hostID := t.vpn.VNIC().ID().Bytes(nil)
		for _, n := range t.vpn.Nodes() {
			p, err := n.HashTable.Set(ctx, t.profile.Key, salt, hostID)
			if err != nil {
				return err
			}
			defer p.Close()
		}
		<-ctx.Done()
		return nil
	})
	eg.Go(func() error {
		stop := timeutil.DefaultTickEmitter.Subscribe(10*time.Second, func(_ timeutil.Time) {
			ds, err := dao.Devices.GetAll(t.store)
			if err != nil {
				log.Println(err)
				return
			}

			for _, d := range ds {
				if d.Id == t.profile.DeviceId {
					continue
				}

				salt := formatSalt(d.Id)
				for _, n := range t.vpn.Nodes() {
					ch, err := n.HashTable.Get(ctx, t.profile.Key.Public, salt)
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
						if t.memes.Has(hostID) {
							continue
						}
						if err := n.PeerExchange.Connect((hostID)); err != nil {
							log.Println(err)
						}
						t.memes.Set(hostID, struct{}{})
					}
				}
			}
		}, nil)
		<-ctx.Done()
		stop()
		return nil
	})
	return eg.Wait()
}

func (t *control) handlePeerAdd(peerID uint64) {
	peer, ok := t.peers.Get(peerID)
	if !ok {
		log.Println("the peer doesn't exist")
		return
	}

	go func() {
		peer.test()
	}()
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
	if p, ok := t.peers.GetAndDelete(id); ok {
		p.close()
	}
}
