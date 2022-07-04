// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
)

// SwarmOptions ...
type SwarmOptions struct {
	Label              string
	ChunkSize          int
	ChunksPerSignature int
	StreamCount        int
	LiveWindow         int
	Integrity          integrity.VerifierOptions
	SchedulingMethod   SchedulingMethod
	DeliveryMode       DeliveryMode
}

// Assign ...
func (o *SwarmOptions) Assign(u SwarmOptions) {
	if u.Label != "" {
		o.Label = u.Label
	}
	if u.ChunkSize != 0 {
		o.ChunkSize = u.ChunkSize
	}
	if u.ChunksPerSignature != 0 {
		o.ChunksPerSignature = u.ChunksPerSignature
	}
	if u.StreamCount != 0 {
		o.StreamCount = u.StreamCount
	}
	if u.LiveWindow != 0 {
		o.LiveWindow = u.LiveWindow
	}
	if u.SchedulingMethod != 0 {
		o.SchedulingMethod = u.SchedulingMethod
	}
	if u.DeliveryMode != 0 {
		o.DeliveryMode = u.DeliveryMode
	}

	o.Integrity.Assign(u.Integrity)
}

// IntegrityVerifierOptions ...
func (o SwarmOptions) IntegrityVerifierOptions() integrity.SwarmVerifierOptions {
	return integrity.SwarmVerifierOptions{
		LiveDiscardWindow:  o.LiveWindow,
		ChunkSize:          o.ChunkSize,
		ChunksPerSignature: o.ChunksPerSignature,
		VerifierOptions:    o.Integrity,
	}
}

// IntegrityWriterOptions ...
func (o SwarmOptions) IntegrityWriterOptions() integrity.SwarmWriterOptions {
	return integrity.SwarmWriterOptions{
		LiveSignatureAlgorithm: o.Integrity.LiveSignatureAlgorithm,
		ProtectionMethod:       o.Integrity.ProtectionMethod,
		ChunkSize:              o.ChunkSize,
		ChunksPerSignature:     o.ChunksPerSignature,
	}
}

// URIOptions ...
func (o SwarmOptions) URIOptions() URIOptions {
	return URIOptions{
		codec.ChunkSizeOption:                        o.ChunkSize,
		codec.ChunksPerSignatureOption:               o.ChunksPerSignature,
		codec.StreamCountOption:                      o.StreamCount,
		codec.ContentIntegrityProtectionMethodOption: int(o.Integrity.ProtectionMethod),
		codec.MerkleHashTreeFunctionOption:           int(o.Integrity.MerkleHashTreeFunction),
		codec.LiveSignatureAlgorithmOption:           int(o.Integrity.LiveSignatureAlgorithm),
	}
}

// NewDefaultSwarmOptions ...
func NewDefaultSwarmOptions() SwarmOptions {
	return SwarmOptions{
		ChunkSize:          1024,
		ChunksPerSignature: 64,
		StreamCount:        1,
		LiveWindow:         1 << 16,
		Integrity:          integrity.NewDefaultVerifierOptions(),
		SchedulingMethod:   PeerSchedulingMethod,
		DeliveryMode:       LowLatencyDeliveryMode,
	}
}
