// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package vnic

import (
	"context"
	"log"
	"syscall/js"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/wasmio"
	"go.uber.org/zap"
)

func init() { RegisterLinkInterface("ws", (*wsLinkCandidate)(nil)) }

var _ Interface = (*wsInterface)(nil)
var _ LinkDialer = (*wsInterface)(nil)
var _ LinkCandidate = (*wsLinkCandidate)(nil)

// NewWSInterface ...
func NewWSInterface(logger *zap.Logger, bridge js.Value) Interface {
	return &wsInterface{
		logger: logger,
		bridge: bridge,
	}
}

// wsInterface ...
type wsInterface struct {
	logger *zap.Logger
	bridge js.Value
}

// ValidScheme ...
func (f *wsInterface) ValidScheme(scheme string) bool {
	return scheme == "ws" || scheme == "wss"
}

func (f *wsInterface) Dial(uri string) (Link, error) {
	log.Println("dialing ws", uri)
	return wasmio.NewWebSocketProxy(f.bridge, uri)
}

func (f *wsInterface) CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error) {
	return &wsLinkCandidate{f, h}, nil
}

type wsLinkCandidate struct {
	iface *wsInterface
	host  *Host
}

func (f *wsLinkCandidate) LocalDescription() (*vnicv1.LinkDescription, error) {
	return nil, nil
}

func (f *wsLinkCandidate) SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error) {
	c, err := f.iface.Dial(d.Description)
	if err != nil {
		return false, err
	}
	f.host.AddLink(c)
	return true, nil
}
