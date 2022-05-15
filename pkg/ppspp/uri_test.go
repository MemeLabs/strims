// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"testing"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/stretchr/testify/assert"
)

func TestURI(t *testing.T) {
	key, err := dao.GenerateKey()
	assert.Nil(t, err)

	uri := &URI{
		ID: key.Public,
		Options: URIOptions{
			codec.ContentIntegrityProtectionMethodOption: int(integrity.ProtectionMethodMerkleTree),
			codec.MerkleHashTreeFunctionOption:           int(integrity.MerkleHashTreeFunctionBLAKE2B256),
			codec.LiveSignatureAlgorithmOption:           int(integrity.LiveSignatureAlgorithmED25519),
			codec.ChunkSizeOption:                        1024,
			codec.ChunksPerSignatureOption:               32,
			codec.StreamCountOption:                      32,
		},
	}

	uri2, err := ParseURI(uri.String())
	assert.Nil(t, err)

	assert.Equal(t, uri, uri2)
}
