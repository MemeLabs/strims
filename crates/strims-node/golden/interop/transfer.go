// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// internal/transfer on the node, for the transfer interop tests. With
// -transfer (and -peer-rpc), every peer's transfer port and RPC pair go
// to transfer's control, as internal/peer gives them, and its binding to
// the network is reported as NetworkPeerOpen, as internal/network
// reports it. The helper's own ppspp wiring is off, so the swarm-run and
// swarm-stop verbs fail: swarms run through announces.
//
// Verbs (byte strings in hex):
//
//	transfer-publish <swarm> <salt>   add the swarm as a transfer and publish it on the network -> <id>
//	transfer-remove <id>              remove a transfer

import (
	"context"
	"encoding/hex"
	"fmt"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/transfer"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

type transferState struct {
	control   transfer.Control
	observers *event.Observers

	mu       sync.Mutex
	nextPeer uint64
}

func newTransferState(logger *zap.Logger, v *vpn.Host) (*transferState, error) {
	// Only local events are emitted, which the cipher never sees.
	sk, err := dao.NewStorageKeyFromBytes(make([]byte, 32), nil)
	if err != nil {
		return nil, err
	}
	q, err := memory.NewTransport().Open("transfer")
	if err != nil {
		return nil, err
	}
	obs := event.NewObservers(logger, q, sk)
	control := transfer.NewControl(context.Background(), logger, v, nil, obs)
	go control.Run()
	return &transferState{control: control, observers: obs}, nil
}

// transferAddPeer gives a new peer to the control, before its server
// listens. The returned id reports the peer's binding and exit.
func (n *node) transferAddPeer(p *vnic.Peer, s *rpc.Server, c rpc.Caller) uint64 {
	t := n.transfer
	t.mu.Lock()
	t.nextPeer++
	pid := t.nextPeer
	t.mu.Unlock()
	t.control.AddPeer(pid, p, s, c)
	return pid
}

func (n *node) transferNetworkPeerOpen(pid uint64) {
	n.transfer.observers.EmitLocal(event.NetworkPeerOpen{PeerID: pid, NetworkKey: n.networkKey})
}

func (n *node) transferRemovePeer(pid uint64) {
	n.transfer.observers.EmitLocal(event.NetworkPeerClose{PeerID: pid, NetworkKey: n.networkKey})
	n.transfer.control.RemovePeer(pid)
}

func (n *node) transferCommand(verb string, args []string) (string, error) {
	if n.transfer == nil {
		return "", fmt.Errorf("%s: transfers are off", verb)
	}
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	control := n.transfer.control
	switch verb {
	case "transfer-publish":
		if err := need(2); err != nil {
			return "", err
		}
		s, err := n.swarm(args[0])
		if err != nil {
			return "", err
		}
		salt, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		id := control.Add(s.swarm, salt)
		control.Publish(id, n.networkKey)
		return hex.EncodeToString(id[:]), nil

	case "transfer-remove":
		if err := need(1); err != nil {
			return "", err
		}
		b, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		id, err := transfer.ParseID(b)
		if err != nil {
			return "", err
		}
		control.Remove(id)
		return "", nil
	}
	return "", fmt.Errorf("unknown command %q", verb)
}
