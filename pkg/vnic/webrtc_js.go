// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package vnic

import (
	"context"
	"encoding/json"
	"syscall/js"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/pointerutil"
	"github.com/MemeLabs/strims/pkg/wasmio"
	"go.uber.org/zap"
)

func init() { RegisterLinkInterface("webrtc", (*webRTCLinkCandidate)(nil)) }

var _ Interface = (*WebRTCInterface)(nil)
var _ LinkCandidate = (*webRTCLinkCandidate)(nil)

// NewWebRTCInterface ...
func NewWebRTCInterface(logger *zap.Logger, bridge js.Value) Interface {
	return &WebRTCInterface{logger, bridge}
}

// WebRTCInterface ...
type WebRTCInterface struct {
	logger *zap.Logger
	bridge js.Value
}

func (d *WebRTCInterface) CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error) {
	pc := wasmio.NewWebRTCProxy(d.bridge)
	pc.CreateDataChannel("data", &wasmio.RTCDataChannelInit{
		ID:         pointerutil.To[uint16](1),
		Negotiated: pointerutil.To(true),
	})

	done := make(chan struct{})
	go func() {
		select {
		case <-done:
		case <-ctx.Done():
			pc.Close()
		}
	}()

	c := &webRTCLinkCandidate{
		ctx:    ctx,
		host:   h,
		bridge: d.bridge,
		pc:     pc,
		done:   done,
	}
	return c, nil
}

type webRTCLinkCandidate struct {
	ctx        context.Context
	host       *Host
	bridge     js.Value
	pc         *wasmio.WebRTCProxy
	remoteDesc bool
	localDesc  bool
	done       chan struct{}
}

func (c *webRTCLinkCandidate) LocalDescription() (d *vnicv1.LinkDescription, err error) {
	var desc *wasmio.RTCSessionDescription
	if c.remoteDesc {
		desc, err = c.pc.CreateAnswer()
	} else {
		desc, err = c.pc.CreateOffer()
	}
	if err != nil {
		return nil, err
	}
	c.pc.SetLocalDescription(desc)
	c.localDesc = true

	for range c.pc.ICECandidates() {
	}

	d = &vnicv1.LinkDescription{
		Interface:   "webrtc",
		Description: c.pc.LocalDescription(),
	}
	return d, nil
}

func (c *webRTCLinkCandidate) SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error) {
	var desc wasmio.RTCSessionDescription
	if err := json.Unmarshal([]byte(d.Description), &desc); err != nil {
		return false, err
	}
	c.pc.SetRemoteDescription(&desc)
	c.remoteDesc = true

	if c.localDesc {
		return c.awaitLink()
	}
	go c.awaitLink()
	return false, nil
}

func (c *webRTCLinkCandidate) awaitLink() (bool, error) {
	defer close(c.done)
	cid, err := c.pc.DataChannelID(c.ctx, "data")
	if err != nil {
		return false, err
	}
	dc, err := wasmio.NewDataChannelProxy(c.bridge, cid)
	if err != nil {
		return false, err
	}
	c.host.AddLink(dc)
	return true, nil
}
