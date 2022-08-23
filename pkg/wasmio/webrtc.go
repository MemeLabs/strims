// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package wasmio

import (
	"context"
	"encoding/json"
	"errors"
	"log"
	"syscall/js"
	"time"

	"github.com/MemeLabs/strims/pkg/sliceutil"
)

const iceGatherTimeout = time.Second * 5
const dataChannelMTU = 64 * 1024

// NewWebRTCProxy ...
func NewWebRTCProxy(bridge js.Value) *WebRTCProxy {
	p := &WebRTCProxy{
		descriptionReady: make(chan any, 1),
		iceCandidates:    make(chan *ICECandidateInit, 32),
		dataChannelIDs:   make(map[string]int),
		dataChannelChs:   make([]chan struct{}, 0),
	}

	proxy := jsObject.New()
	proxy.Set("onicecandidate", p.funcs.Register(js.FuncOf(p.onICECandidate)))
	proxy.Set("onconnectionstatechange", p.funcs.Register(js.FuncOf(p.onConnectionStateChange)))
	proxy.Set("onicegatheringstatechange", p.funcs.Register(js.FuncOf(p.onICEGatheringStateChange)))
	proxy.Set("onsignalingstatechange", p.funcs.Register(js.FuncOf(p.onSignalingStateChange)))
	proxy.Set("oncreateoffer", p.funcs.Register(js.FuncOf(p.onCreateOffer)))
	proxy.Set("oncreateanswer", p.funcs.Register(js.FuncOf(p.onCreateAnswer)))
	proxy.Set("ondatachannel", p.funcs.Register(js.FuncOf(p.onDataChannel)))
	p.proxy = bridge.Call("openWebRTC", proxy)

	return p
}

// WebRTCProxy ...
type WebRTCProxy struct {
	proxy             js.Value
	funcs             Funcs
	descriptionReady  chan any
	iceCandidates     chan *ICECandidateInit
	localDescription  string
	connectionState   string
	iceGatheringState string
	signalingState    string
	dataChannelIDs    map[string]int
	dataChannelChs    []chan struct{}
}

// CreateOffer ...
func (p *WebRTCProxy) CreateOffer() (*RTCSessionDescription, error) {
	p.proxy.Call("createOffer")
	return selectRTCSessionDescription(p.descriptionReady)
}

// CreateAnswer ...
func (p *WebRTCProxy) CreateAnswer() (*RTCSessionDescription, error) {
	p.proxy.Call("createAnswer")
	return selectRTCSessionDescription(p.descriptionReady)
}

func selectRTCSessionDescription(ch chan any) (*RTCSessionDescription, error) {
	select {
	case ri := <-ch:
		switch r := ri.(type) {
		case error:
			return nil, r
		case *RTCSessionDescription:
			return r, nil
		default:
			log.Panicf("expected *RTCSessionDescription got %T", r)
		}
	case <-time.After(10 * time.Second):
		return nil, ErrOperationTimeout
	}

	panic("unexpted state")
}

// CreateDataChannel ...
func (p *WebRTCProxy) CreateDataChannel(label string, options *RTCDataChannelInit) {
	s, _ := json.Marshal(options)
	p.proxy.Call("createDataChannel", label, string(s))
}

// AddICECandidate ...
func (p *WebRTCProxy) AddICECandidate(candidate *ICECandidateInit) {
	s, _ := json.Marshal(candidate)
	p.proxy.Call("addIceCandidate", string(s))
}

// SetLocalDescription ...
func (p *WebRTCProxy) SetLocalDescription(description *RTCSessionDescription) {
	s, _ := json.Marshal(description)
	p.proxy.Call("setLocalDescription", string(s))
}

// SetRemoteDescription ...
func (p *WebRTCProxy) SetRemoteDescription(description *RTCSessionDescription) {
	s, _ := json.Marshal(description)
	p.proxy.Call("setRemoteDescription", string(s))
}

// ICECandidates ...
func (p *WebRTCProxy) ICECandidates() <-chan *ICECandidateInit {
	return p.iceCandidates
}

// LocalDescription ...
func (p *WebRTCProxy) LocalDescription() string {
	return p.localDescription
}

// ConnectionState ...
func (p *WebRTCProxy) ConnectionState() string {
	return p.connectionState
}

// ICEGatheringState ...
func (p *WebRTCProxy) ICEGatheringState() string {
	return p.iceGatheringState
}

// SignalingState ...
func (p *WebRTCProxy) SignalingState() string {
	return p.signalingState
}

// Close ...
func (p *WebRTCProxy) Close() {
	p.proxy.Call("close")
	p.funcs.Release()
}

func (p *WebRTCProxy) onICECandidate(this js.Value, args []js.Value) any {
	// log.Println("ice candidate", args[0].String())
	p.localDescription = args[1].String()

	cs := args[0].String()
	if cs == "null" {
		p.iceCandidates <- nil
		close(p.iceCandidates)
		return nil
	}

	c := &ICECandidateInit{}
	if err := json.Unmarshal([]byte(cs), c); err != nil {
		log.Panicln("failed to parse ice candidate", err)
	}
	p.iceCandidates <- c
	return nil
}

func (p *WebRTCProxy) onConnectionStateChange(this js.Value, args []js.Value) any {
	// log.Println("connection state", args[0].String())
	p.connectionState = args[0].String()
	return nil
}

func (p *WebRTCProxy) onICEGatheringStateChange(this js.Value, args []js.Value) any {
	// log.Println("ice gather state", args[0].String())
	p.iceGatheringState = args[0].String()
	return nil
}

func (p *WebRTCProxy) onSignalingStateChange(this js.Value, args []js.Value) any {
	// log.Println("signaling state", args[0].String())
	p.signalingState = args[0].String()
	return nil
}

func (p *WebRTCProxy) onCreateOffer(this js.Value, args []js.Value) any {
	sendRTCSessionDescription(p.descriptionReady, args)
	return nil
}

func (p *WebRTCProxy) onCreateAnswer(this js.Value, args []js.Value) any {
	sendRTCSessionDescription(p.descriptionReady, args)
	return nil
}

func sendRTCSessionDescription(ch chan any, args []js.Value) {
	if err := args[0]; !err.IsUndefined() {
		ch <- errors.New(err.String())
		return
	}

	c := &RTCSessionDescription{}
	if err := json.Unmarshal([]byte(args[1].String()), c); err != nil {
		ch <- err
		return
	}
	ch <- c
}

func (p *WebRTCProxy) onDataChannel(this js.Value, args []js.Value) any {
	// log.Println("data channel", args[0].Int(), args[1].String())
	p.dataChannelIDs[args[1].String()] = args[0].Int()

	for _, ch := range p.dataChannelChs {
		select {
		case ch <- struct{}{}:
		default:
		}
	}
	return nil
}

// DataChannelID ...
func (p *WebRTCProxy) DataChannelID(ctx context.Context, label string) (int, error) {
	if id, ok := p.dataChannelIDs[label]; ok {
		return id, nil
	}

	ch := make(chan struct{}, 1)
	p.dataChannelChs = append(p.dataChannelChs, ch)
	defer func() { p.dataChannelChs = sliceutil.Remove(p.dataChannelChs, ch) }()

	for {
		select {
		case <-ch:
			if id, ok := p.dataChannelIDs[label]; ok {
				return id, nil
			}
		case <-ctx.Done():
			return 0, ctx.Err()
		}
	}
}

// DataChannelProxy ...
type DataChannelProxy interface {
	MTU() int
	Write(b []byte) (int, error)
	Read(b []byte) (n int, err error)
	Close() error
}

// NewDataChannelProxy ...
func NewDataChannelProxy(bridge js.Value, id int) (DataChannelProxy, error) {
	return newChannel(dataChannelMTU, bridge, "openDataChannel", id)
}

// RTCSessionDescription ...
type RTCSessionDescription struct {
	Type string `json:"type"`
	SDP  string `json:"sdp"`
}

// ICECandidateInit ...
type ICECandidateInit struct {
	Candidate        string  `json:"candidate"`
	SDPMid           *string `json:"sdpMid,omitempty"`
	SDPMLineIndex    *uint16 `json:"sdpMLineIndex,omitempty"`
	UsernameFragment string  `json:"usernameFragment"`
}

// RTCDataChannelInit ...
type RTCDataChannelInit struct {
	ID                *uint16 `json:"id,omitempty"`
	Ordered           *bool   `json:"ordered,omitempty"`
	MaxPacketLifeTime *uint16 `json:"maxPacketLifeTime,omitempty"`
	MaxRetransmits    *uint16 `json:"maxRetransmits,omitempty"`
	Protocol          *string `json:"protocol,omitempty"`
	Negotiated        *bool   `json:"negotiated,omitempty"`
}
