// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

// WebRTCScheme webrtc url scheme
const WebRTCScheme = "webrtc"

// NewWebRTCInterface ...
func NewWebRTCInterface(d *WebRTCDialer) *WebRTCInterface {
	return &WebRTCInterface{d}
}

// WebRTCInterface ...
type WebRTCInterface struct {
	Dialer *WebRTCDialer
}

// ValidScheme ...
func (w *WebRTCInterface) ValidScheme(scheme string) bool {
	return scheme == WebRTCScheme
}

// Dial ...
func (w *WebRTCInterface) Dial(addr InterfaceAddr) (Link, error) {
	return w.Dialer.Dial(addr.(WebRTCMediator))
}

// WebRTCMediator ...
type WebRTCMediator interface {
	InterfaceAddr
	GetOffer() ([]byte, error)
	GetAnswer() ([]byte, error)
	GetICECandidates() <-chan []byte
	SendOffer([]byte) error
	SendAnswer([]byte) error
	SendICECandidate([]byte) error
}
