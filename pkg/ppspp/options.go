package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
)

// SwarmOptions ...
type SwarmOptions struct {
	ChunkSize          int
	ChunksPerSignature int
	LiveWindow         int
	Integrity          integrity.VerifierOptions
	SchedulingMethod   SchedulingMethod
}

// Assign ...
func (o *SwarmOptions) Assign(u SwarmOptions) {
	if u.ChunkSize != 0 {
		o.ChunkSize = u.ChunkSize
	}
	if u.ChunksPerSignature != 0 {
		o.ChunksPerSignature = u.ChunksPerSignature
	}
	if u.LiveWindow != 0 {
		o.LiveWindow = u.LiveWindow
	}
	if u.SchedulingMethod != 0 {
		o.SchedulingMethod = u.SchedulingMethod
	}

	o.Integrity.Assign(u.Integrity)
}

// URIOptions ...
func (o SwarmOptions) URIOptions() URIOptions {
	return URIOptions{
		codec.ChunkSizeOption:                        o.ChunkSize,
		codec.ChunksPerSignatureOption:               o.ChunksPerSignature,
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
		LiveWindow:         1 << 16,
		Integrity:          integrity.NewDefaultVerifierOptions(),
		SchedulingMethod:   PeerSchedulingMethod,
	}
}
