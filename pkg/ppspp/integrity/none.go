// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

const NoneSignatureSize = 0

// NewNoneSigner ...
func NewNoneSigner() *NoneSigner {
	return &NoneSigner{}
}

// NoneSigner ...
type NoneSigner struct{}

// Sign ...
func (s *NoneSigner) Sign(t timeutil.Time, p []byte) []byte {
	return nil
}

// Size ...
func (s *NoneSigner) Size() int {
	return NoneSignatureSize
}

// NewNoneVerifier ...
func NewNoneVerifier() *NoneVerifier {
	return &NoneVerifier{}
}

// NoneVerifier ...
type NoneVerifier struct{}

// Verify ...
func (s *NoneVerifier) Verify(t timeutil.Time, p []byte, sig []byte) bool {
	return true
}

// Size ...
func (s *NoneVerifier) Size() int {
	return NoneSignatureSize
}

var noneChannelVerifier = &NoneChannelVerifier{}
var noneChunkVerifier = &NoneChunkVerifier{}

// NoneSwarmVerifier ...
type NoneSwarmVerifier struct{}

// WriteIntegrity ...
func (v *NoneSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error) {
	return 0, nil
}

// ChannelVerifier ...
func (v *NoneSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return noneChannelVerifier
}

func (v *NoneSwarmVerifier) ImportCache(c *swarmpb.Cache) error {
	return nil
}

func (v *NoneSwarmVerifier) ExportCache() *swarmpb.Cache_Integrity {
	return &swarmpb.Cache_Integrity{}
}

func (v *NoneSwarmVerifier) Reset() {}

// NoneChannelVerifier ...
type NoneChannelVerifier struct{}

// ChunkVerifier ...
func (v *NoneChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	return noneChunkVerifier
}

// NoneChunkVerifier ...
type NoneChunkVerifier struct{}

// SetSignedIntegrity ...
func (v *NoneChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts timeutil.Time, sig []byte) {}

// SetIntegrity ...
func (v *NoneChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {}

// Verify ...
func (v *NoneChunkVerifier) Verify(b binmap.Bin, d []byte) (bool, error) {
	return true, nil
}
