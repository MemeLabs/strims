// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// Profile replication for the replication interop tests, wired as
// internal/session and internal/app wire it: a profile store on a memory
// kv with an event queue, internal/replication's control on the node's
// vpn host, and every peer's RPC pair given to the control with a
// PeerAdd event (internal/peer's part). The profile's key is the node's
// profile key, so the node's peers that hold it are the profile's other
// devices.
//
// Verbs (byte strings in hex):
//
//	repl-new <os>              create the profile as its first device
//	repl-token                 a pairing token for a new device -> <token>
//	repl-import <token>        become the device a pairing token makes
//	repl-highlight <alias>     add a chat highlight -> <id>
//	repl-edit <id> <alias>     rename a highlight
//	repl-delete-device <id>    delete a device
//	repl-highlights            -> <"id:alias" lines, by id>
//	repl-devices               -> <device ids, comma-separated>
//
// Replication must be set up before the node links to the profile's
// other devices.

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"sort"
	"strconv"
	"strings"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/dao/versionvector"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/replication"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/kv/kvtest"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/vnic"
	"google.golang.org/protobuf/proto"
)

type replState struct {
	mu        sync.Mutex
	store     *dao.ReplicatedStore
	observers *event.Observers
	control   replication.Control
	key       []byte
	nextPeer  uint64
}

// replStart opens the profile store and runs replication on it. The dump
// a bootstrap sends reads the chat UI config, which must exist, and
// applying a replicated record compares versions, which must be set, so
// an empty, versioned config is written where there is none, without
// replicating it.
func (n *node) replStart(id uint64, key []byte) error {
	sk, err := dao.NewStorageKeyFromBytes(key, nil)
	if err != nil {
		return err
	}
	plain := dao.NewProfileStore(id, sk, n.replBlob, nil)
	if _, err := dao.ChatUIConfig.Get(plain); errors.Is(err, kv.ErrRecordNotFound) {
		if err := dao.ChatUIConfig.Set(plain, &chatv1.UIConfig{Version: versionvector.New()}); err != nil {
			return err
		}
	} else if err != nil {
		return err
	}
	q, err := memory.NewTransport().Open(strconv.FormatUint(id, 10))
	if err != nil {
		return err
	}
	obs := event.NewObservers(n.logger, q, sk)
	ps := dao.NewProfileStore(id, sk, n.replBlob, &dao.ProfileStoreOptions{EventEmitter: dao.EventEmitterFunc(obs.EmitGlobal)})
	rs, err := dao.NewReplicatedStore(ps)
	if err != nil {
		return err
	}
	profile, err := dao.Profile.Get(rs)
	if err != nil {
		return err
	}
	control := replication.NewControl(context.Background(), n.logger, n.vpnHost, rs, obs, profile)
	n.repl = &replState{store: rs, observers: obs, control: control, key: key}
	go control.Run()
	return nil
}

// replAddPeer gives a new peer's RPC pair to replication.
func (n *node) replAddPeer(id kademlia.ID, p *vnic.Peer, s *rpc.Server, c rpc.Caller) {
	r := n.repl
	if r == nil {
		return
	}
	r.mu.Lock()
	r.nextPeer++
	pid := r.nextPeer
	r.mu.Unlock()
	r.control.AddPeer(pid, p, s, c)
	r.observers.EmitLocal(event.PeerAdd{ID: pid, HostID: id})
	go func() {
		<-p.Done()
		r.control.RemovePeer(pid)
		r.observers.EmitLocal(event.PeerRemove{ID: pid, HostID: id})
	}()
}

func (n *node) replCommand(verb string, args []string) (string, error) {
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	unhex := func(s string) ([]byte, error) {
		if s == "-" {
			return nil, nil
		}
		return hex.DecodeString(s)
	}

	switch verb {
	case "repl-new":
		if err := need(1); err != nil {
			return "", err
		}
		os, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		profile, err := dao.NewProfile("go")
		if err != nil {
			return "", err
		}
		profile.Key = n.profileKey
		key := make([]byte, dao.KeySize)
		if _, err := rand.Read(key); err != nil {
			return "", err
		}
		user := &authv1.ServerUserThing{
			Name: "go",
			Credentials: &authv1.ServerUserThing_Unencrypted_{
				Unencrypted: &authv1.ServerUserThing_Unencrypted{ProfileId: profile.Id, ProfileKey: key},
			},
		}
		ps, err := dao.StoreProfileThing(n.replBlob, user, profile, key)
		if err != nil {
			return "", err
		}
		device, err := dao.NewDevice(dao.ProfileID.IDGenerator(ps), "", string(os))
		if err != nil {
			return "", err
		}
		if err := dao.Devices.Insert(ps, device); err != nil {
			return "", err
		}
		if _, err := dao.Profile.Transform(ps, func(p *profilev1.Profile) error {
			p.DeviceId = device.Id
			return nil
		}); err != nil {
			return "", err
		}
		if err := n.replStart(profile.Id, key); err != nil {
			return "", err
		}
		netID, err := n.repl.store.GenerateID()
		if err != nil {
			return "", err
		}
		network := &networkv1.Network{
			Id:          netID,
			Version:     versionvector.New(),
			Certificate: &certificate.Certificate{Key: n.networkKey, NotAfter: 1 << 62},
			Alias:       "net",
		}
		return "", dao.Networks.Insert(n.repl.store, network)

	case "repl-token":
		r := n.repl
		if r == nil {
			return "", errors.New("replication not set up")
		}
		profile, err := dao.Profile.Get(r.store)
		if err != nil {
			return "", err
		}
		user, err := dao.GetServerAuthThing(n.replBlob, profile.Name)
		if err != nil {
			return "", err
		}
		profile = proto.Clone(profile).(*profilev1.Profile)
		if profile.DeviceId, err = r.store.GenerateID(); err != nil {
			return "", err
		}
		networks, err := dao.Networks.GetAll(r.store)
		if err != nil {
			return "", err
		}
		for _, nw := range networks {
			nw.ServerConfig = nil
		}
		bootstraps, err := dao.BootstrapClients.GetAll(r.store)
		if err != nil {
			return "", err
		}
		devices, err := dao.Devices.GetAll(r.store)
		if err != nil {
			return "", err
		}
		ids, err := dao.ProfileID.Pop(r.store, 1000)
		if err != nil {
			return "", err
		}
		token := &authv1.PairingToken{
			Auth:       user,
			Profile:    profile,
			Networks:   networks,
			Bootstraps: bootstraps,
			Devices:    devices,
			ProfileId:  ids,
		}
		b, err := proto.Marshal(token)
		if err != nil {
			return "", err
		}
		return hex.EncodeToString(b), nil

	case "repl-import":
		if err := need(1); err != nil {
			return "", err
		}
		b, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		token := &authv1.PairingToken{}
		if err := proto.Unmarshal(b, token); err != nil {
			return "", err
		}
		id, key, err := dao.OpenServerAuthThing(token.Auth, "")
		if err != nil {
			return "", err
		}
		if err := dao.ImportPairingToken(n.replBlob, token, key); err != nil {
			return "", err
		}
		// Replication's device init updates this device's record, which
		// has to exist.
		sk, err := dao.NewStorageKeyFromBytes(key, nil)
		if err != nil {
			return "", err
		}
		plain := dao.NewProfileStore(id, sk, n.replBlob, nil)
		device := &profilev1.Device{Id: token.Profile.DeviceId, Version: versionvector.New(), Os: "go"}
		if err := dao.Devices.Insert(plain, device); err != nil {
			return "", err
		}
		return "", n.replStart(id, key)

	case "repl-highlight":
		if err := need(1); err != nil {
			return "", err
		}
		alias, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		h, err := dao.NewChatUIConfigHighlight(n.repl.store, string(alias), alias)
		if err != nil {
			return "", err
		}
		if err := dao.ChatUIConfigHighlights.Insert(n.repl.store, h); err != nil {
			return "", err
		}
		return strconv.FormatUint(h.Id, 10), nil

	case "repl-edit":
		if err := need(2); err != nil {
			return "", err
		}
		id, err := strconv.ParseUint(args[0], 10, 64)
		if err != nil {
			return "", err
		}
		alias, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		_, err = dao.ChatUIConfigHighlights.Transform(n.repl.store, id, func(h *chatv1.UIConfigHighlight) error {
			h.Alias = string(alias)
			return nil
		})
		return "", err

	case "repl-delete-device":
		if err := need(1); err != nil {
			return "", err
		}
		id, err := strconv.ParseUint(args[0], 10, 64)
		if err != nil {
			return "", err
		}
		return "", dao.Devices.Delete(n.repl.store, id)

	case "repl-highlights":
		hs, err := dao.ChatUIConfigHighlights.GetAll(n.repl.store)
		if err != nil {
			return "", err
		}
		sort.Slice(hs, func(i, j int) bool { return hs[i].Id < hs[j].Id })
		var lines []string
		for _, h := range hs {
			lines = append(lines, fmt.Sprintf("%d:%s", h.Id, h.Alias))
		}
		return hexs(strings.Join(lines, "\n")), nil

	case "repl-devices":
		ds, err := dao.Devices.GetAll(n.repl.store)
		if err != nil {
			return "", err
		}
		var ids []string
		for _, d := range ds {
			ids = append(ids, strconv.FormatUint(d.Id, 10))
		}
		sort.Strings(ids)
		return hexs(strings.Join(ids, ",")), nil
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

// replBlobStore is the helper's kv for profile stores.
func newReplBlob() *kvtest.Store {
	return kvtest.NewMemStore().(*kvtest.Store)
}
