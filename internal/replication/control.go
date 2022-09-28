// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"context"
	"log"
	"strconv"

	"github.com/MemeLabs/strims/internal/api"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/notification"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

type Control interface {
	Run()
	AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer
	RemovePeer(id uint64)
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
	peers  syncutil.Map[uint64, *peer]
}

// Run ...
func (t *control) Run() {
	// publish that we're available to the dht
	// search for announcements and connect to replication candidates

	for {
		select {
		case e := <-t.events:
			switch e := e.(type) {
			case event.PeerAdd:
				go t.handlePeerAdd(e.ID)
			}
		case <-t.ctx.Done():
			return
		}
	}
}

func formatSalt(replicaID uint32) []byte {
	return strconv.AppendUint([]byte("replication:"), uint64(replicaID), 10)
}

func (t *control) doFoo() error {
	mu := dao.NewMutex(t.logger, t.store, "replication")
	for {
		ctx, err := mu.Lock(t.ctx)
		if err != nil {
			return err
		}

		salt := formatSalt(0)
		hostID := t.vpn.VNIC().ID().Bytes(nil)

		eg, ctx := errgroup.WithContext(ctx)
		eg.Go(func() error {
			for _, n := range t.vpn.Nodes() {
				p, err := n.HashTable.Set(ctx, t.profile.Key, salt, hostID)
				if err != nil {
					return err
				}
				defer p.Close()
			}
			return nil
		})
		eg.Go(func() error {
			salt := formatSalt(1)
			for _, n := range t.vpn.Nodes() {
				ch, err := n.HashTable.Get(ctx, t.profile.Key.Public, salt)
				if err != nil {
					return err
				}
				for hostID := range ch {
					log.Println(kademlia.UnmarshalID(hostID))
				}
			}
			return nil
		})
		if err := eg.Wait(); err != nil {
			return err
		}
	}
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
func (t *control) AddPeer(id uint64, vnicPeer *vnic.Peer, client api.PeerClient) Peer {
	p := newPeer(id, vnicPeer, client, t.logger, t.observers, t.vpn, t.qosc)
	t.peers.Set(p.id, p)
	return p
}

// RemovePeer ...
func (t *control) RemovePeer(id uint64) {
	if p, ok := t.peers.GetAndDelete(id); ok {
		p.close()
	}
}
