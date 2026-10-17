// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// internal/network on the node, for the network membership interop
// tests. With -network-control (and -peer-rpc), every peer's RPC pair
// goes to internal/network's control as internal/peer gives it, with a
// PeerAdd event, and peers bind to networks by negotiation: nothing binds
// them on the fixed port pair. The networks live in a profile store of
// their own, written through the dao as internal/frontend writes it.
//
// Verbs (byte strings in hex):
//
//	net-create <name>                               create a network and run its CA -> <id> <key>
//	net-invite <id>                                 an invitation to the network -> <invitation>
//	net-join <invitation>                           join a network from an invitation -> <id> <key>
//	net-alias <id> <alias>                          rename on the network, which renews
//	net-cert <id>                                   -> <trusted 0|1> <subject> <renewal error code>
//	net-delete <id>                                 leave the network
//	net-handler <key> <port>                        report the network's messages to port as msg lines
//	net-send <key> <dst> <dst port> <src port> <body>
//
// Events:
//
//	net-open <host id> <network key>
//	net-close <host id> <network key>

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"strconv"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/transfer"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/kv/kvtest"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// noNotifications drops the control's notifications.
type noNotifications struct{}

func (noNotifications) Watch(ctx context.Context) <-chan *notificationv1.Event {
	return make(chan *notificationv1.Event)
}
func (noNotifications) Dispatch(*notificationv1.Notification) error { return nil }
func (noNotifications) Dismiss(uint64) error                        { return nil }

type networkState struct {
	store     *dao.ReplicatedStore
	observers *event.Observers
	control   network.Control
	profile   *profilev1.Profile
	vpnHost   *vpn.Host

	mu       sync.Mutex
	nextPeer uint64
	hosts    map[uint64]kademlia.ID
}

// newNetworkState opens a profile store for the profile key, and runs
// internal/network's control on it.
func newNetworkState(logger *zap.Logger, v *vpn.Host, profileKey *key.Key, label string) (*networkState, error) {
	profile, err := dao.NewProfile(label)
	if err != nil {
		return nil, err
	}
	profile.Key = profileKey
	key := make([]byte, dao.KeySize)
	if _, err := rand.Read(key); err != nil {
		return nil, err
	}
	blob := kvtest.NewMemStore().(*kvtest.Store)
	user := &authv1.ServerUserThing{
		Name: label,
		Credentials: &authv1.ServerUserThing_Unencrypted_{
			Unencrypted: &authv1.ServerUserThing_Unencrypted{ProfileId: profile.Id, ProfileKey: key},
		},
	}
	ps, err := dao.StoreProfileThing(blob, user, profile, key)
	if err != nil {
		return nil, err
	}
	device, err := dao.NewDevice(dao.ProfileID.IDGenerator(ps), "", "linux")
	if err != nil {
		return nil, err
	}
	if err := dao.Devices.Insert(ps, device); err != nil {
		return nil, err
	}
	profile, err = dao.Profile.Transform(ps, func(p *profilev1.Profile) error {
		p.DeviceId = device.Id
		return nil
	})
	if err != nil {
		return nil, err
	}
	sk, err := dao.NewStorageKeyFromBytes(key, nil)
	if err != nil {
		return nil, err
	}
	q, err := memory.NewTransport().Open(strconv.FormatUint(profile.Id, 10))
	if err != nil {
		return nil, err
	}
	obs := event.NewObservers(logger, q, sk)
	store := dao.NewProfileStore(profile.Id, sk, blob, &dao.ProfileStoreOptions{EventEmitter: dao.EventEmitterFunc(obs.EmitGlobal)})
	rs, err := dao.NewReplicatedStore(store)
	if err != nil {
		return nil, err
	}
	ctx := context.Background()
	tc := transfer.NewControl(ctx, logger, v, nil, obs)
	go tc.Run()
	control := network.NewControl(ctx, logger, v, rs, obs, network.NewBroker(logger), profile, tc, noNotifications{})
	s := &networkState{
		store:     rs,
		observers: obs,
		control:   control,
		profile:   profile,
		vpnHost:   v,
		hosts:     map[uint64]kademlia.ID{},
	}
	go control.Run()
	go s.report()
	return s, nil
}

// report emits the peers' network bindings.
func (s *networkState) report() {
	events, done := s.observers.Events()
	defer done()
	for e := range events {
		switch e := e.(type) {
		case event.NetworkPeerOpen:
			emit("net-open %s %s", s.host(e.PeerID), hex.EncodeToString(e.NetworkKey))
		case event.NetworkPeerClose:
			emit("net-close %s %s", s.host(e.PeerID), hex.EncodeToString(e.NetworkKey))
		}
	}
}

func (s *networkState) host(pid uint64) string {
	s.mu.Lock()
	defer s.mu.Unlock()
	return idHex(s.hosts[pid])
}

// networkAddPeer gives a new peer to the control, before its server
// listens, and reports it with a PeerAdd event, as internal/peer does.
func (n *node) networkAddPeer(id kademlia.ID, p *vnic.Peer, srv *rpc.Server, c rpc.Caller) {
	s := n.network
	s.mu.Lock()
	s.nextPeer++
	pid := s.nextPeer
	s.hosts[pid] = id
	s.mu.Unlock()
	s.control.AddPeer(pid, p, srv, c)
	go func() {
		s.observers.EmitLocal(event.PeerAdd{ID: pid, HostID: id})
		<-p.Done()
		s.control.RemovePeer(pid)
		s.observers.EmitLocal(event.PeerRemove{ID: pid, HostID: id})
	}()
}

// certificateTrusted is the membership check internal/network makes: the
// network's root signed it, and it hasn't expired.
func certificateTrusted(c *certificate.Certificate) bool {
	return bytes.Equal(dao.CertificateNetworkKey(c), c.GetParent().GetKey()) &&
		!timeutil.Now().After(timeutil.Unix(int64(c.GetNotAfter()), 0))
}

func (n *node) networkCommand(verb string, args []string) (string, error) {
	s := n.network
	if s == nil {
		return "", fmt.Errorf("%s: network control is off", verb)
	}
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	networkID := func(i int) (uint64, error) {
		if err := need(i + 1); err != nil {
			return 0, err
		}
		return strconv.ParseUint(args[i], 10, 64)
	}
	vpnNode := func(i int) (*vpn.Node, error) {
		if err := need(i + 1); err != nil {
			return nil, err
		}
		key, err := hex.DecodeString(args[i])
		if err != nil {
			return nil, err
		}
		node, ok := s.vpnHost.Node(key)
		if !ok {
			return nil, errors.New("network not running")
		}
		return node, nil
	}
	switch verb {
	case "net-create":
		if err := need(1); err != nil {
			return "", err
		}
		name, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		nw, err := dao.NewNetwork(s.store, string(name), nil, s.profile)
		if err != nil {
			return "", err
		}
		if err := dao.Networks.Insert(s.store, nw); err != nil {
			return "", err
		}
		return fmt.Sprintf("%d %s", nw.Id, hex.EncodeToString(dao.NetworkKey(nw))), nil

	case "net-invite":
		id, err := networkID(0)
		if err != nil {
			return "", err
		}
		nw, err := dao.Networks.Get(s.store, id)
		if err != nil {
			return "", err
		}
		inv, err := dao.NewInvitationV0(s.profile.Key, nw.Certificate, nil)
		if err != nil {
			return "", err
		}
		data, err := proto.Marshal(inv)
		if err != nil {
			return "", err
		}
		b, err := proto.Marshal(&networkv1.Invitation{Version: 0, Data: data})
		if err != nil {
			return "", err
		}
		return hex.EncodeToString(b), nil

	case "net-join":
		if err := need(1); err != nil {
			return "", err
		}
		b, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		var wrapper networkv1.Invitation
		if err := proto.Unmarshal(b, &wrapper); err != nil {
			return "", err
		}
		var inv networkv1.InvitationV0
		if err := proto.Unmarshal(wrapper.Data, &inv); err != nil {
			return "", err
		}
		nw, err := dao.NewNetworkFromInvitationV0(s.store, &inv, s.profile)
		if err != nil {
			return "", err
		}
		if err := dao.Networks.Insert(s.store, nw); err != nil {
			return "", err
		}
		return fmt.Sprintf("%d %s", nw.Id, hex.EncodeToString(dao.NetworkKey(nw))), nil

	case "net-alias":
		id, err := networkID(0)
		if err != nil {
			return "", err
		}
		if err := need(2); err != nil {
			return "", err
		}
		alias, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		return "", s.control.SetAlias(id, string(alias))

	case "net-cert":
		id, err := networkID(0)
		if err != nil {
			return "", err
		}
		nw, err := dao.Networks.Get(s.store, id)
		if err != nil {
			return "", err
		}
		trusted := 0
		if certificateTrusted(nw.Certificate) {
			trusted = 1
		}
		return fmt.Sprintf("%d %s %d", trusted, hexs(nw.Certificate.Subject), nw.CertificateRenewalError), nil

	case "net-delete":
		id, err := networkID(0)
		if err != nil {
			return "", err
		}
		return "", dao.Networks.Delete(s.store, id)

	case "net-handler":
		node, err := vpnNode(0)
		if err != nil {
			return "", err
		}
		if err := need(2); err != nil {
			return "", err
		}
		port, err := parseUint(args[1], 16)
		if err != nil {
			return "", err
		}
		return "", node.Network.SetHandler(uint16(port), vpn.MessageHandlerFunc(func(m *vpn.Message) error {
			emit("msg %s %d %d %d %d %s", idHex(m.SrcHostID()), m.Header.SrcPort, m.Header.DstPort, m.Trailer.Hops, m.Header.Flags, hex.EncodeToString(m.Body))
			return nil
		}))

	case "net-send":
		node, err := vpnNode(0)
		if err != nil {
			return "", err
		}
		if err := need(5); err != nil {
			return "", err
		}
		dst, err := parseID(args[1])
		if err != nil {
			return "", err
		}
		dp, err := parseUint(args[2], 16)
		if err != nil {
			return "", err
		}
		sp, err := parseUint(args[3], 16)
		if err != nil {
			return "", err
		}
		body, err := unhex(args[4])
		if err != nil {
			return "", err
		}
		return "", node.Network.Send(dst, uint16(dp), uint16(sp), body)
	}
	return "", fmt.Errorf("unknown command %q", verb)
}
