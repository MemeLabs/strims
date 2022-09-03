// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	"bytes"
	"crypto/rand"
	"crypto/sha256"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/blake2b"
)

type discardWriter struct{}

func (d *discardWriter) Write(p []byte) (int, error) {
	return len(p), nil
}

func (d *discardWriter) Flush() error {
	return nil
}

func (d *discardWriter) Reset() {}

func TestMerkleWriter(t *testing.T) {
	key, err := dao.GenerateKey()
	if err != nil {
		t.Fatal(err)
	}

	liveDiscardWindow := 1 << 10 // 1mb
	chunksPerSignature := 32
	chunkSize := 1024
	w := NewMerkleWriter(&MerkleWriterOptions{
		Verifier: NewMerkleSwarmVerifier(&MerkleOptions{
			LiveDiscardWindow:  liveDiscardWindow,
			ChunksPerSignature: chunksPerSignature,
			ChunkSize:          chunkSize,
			Verifier:           NewED25519Verifier(key.Public),
			Hash:               sha256.New,
		}),
		Writer:             &discardWriter{},
		ChunksPerSignature: chunksPerSignature,
		ChunkSize:          chunkSize,
		Signer:             NewED25519Signer(key.Private),
	})

	r, err := os.OpenFile("/dev/urandom", os.O_RDONLY, 0644)
	if err != nil {
		t.Fatal(err)
	}

	_, err = io.CopyN(w, r, 1<<21) // 2 mb
	assert.Nil(t, err, "write should not fail")
}

func TestMerkleVerifier(t *testing.T) {
	key, err := dao.GenerateKey()
	assert.Nil(t, err)

	const liveDiscardWindow = 1 << 10
	const chunksPerSignature = 32
	const chunkSize = 1024
	const n = 1 << 20

	assert.Equal(t, n, liveDiscardWindow*chunkSize)

	v0 := NewMerkleSwarmVerifier(&MerkleOptions{
		LiveDiscardWindow:  liveDiscardWindow,
		ChunksPerSignature: chunksPerSignature,
		ChunkSize:          chunkSize,
		Verifier:           NewED25519Verifier(key.Public),
		Hash:               blake2bFunc(blake2b.New256),
	})

	var b bytes.Buffer
	w := NewMerkleWriter(&MerkleWriterOptions{
		Verifier:           v0,
		Writer:             bufioutil.NewWriter(&b, 1024),
		ChunksPerSignature: chunksPerSignature,
		ChunkSize:          chunkSize,
		Signer:             NewED25519Signer(key.Private),
	})
	io.CopyN(w, rand.Reader, n)
	d := b.Bytes()

	verify := func(t *testing.T, src *MerkleSwarmVerifier) *MerkleSwarmVerifier {
		dst := NewMerkleSwarmVerifier(&MerkleOptions{
			LiveDiscardWindow:  liveDiscardWindow,
			ChunksPerSignature: chunksPerSignature,
			ChunkSize:          chunkSize,
			Verifier:           NewED25519Verifier(key.Public),
			Hash:               blake2bFunc(blake2b.New256),
		})

		cv := dst.ChannelVerifier()
		for i := 0; i < n/chunkSize; i++ {
			b := binmap.Bin(i * 2)
			v := cv.ChunkVerifier(b)

			var w testWriter
			_, err := src.WriteIntegrity(b, binmap.New(), &w)
			assert.Nil(t, err, fmt.Sprintf("failed to write integrity mesages bin %d", b))

			v.SetSignedIntegrity(b, w.SignedIntegrity.Timestamp.Time, w.SignedIntegrity.Signature)
			for _, m := range w.Integrity {
				v.SetIntegrity(m.Address.Bin(), m.Hash)
			}
			ok, err := v.Verify(b, d[i*chunkSize:(i+1)*chunkSize])
			assert.Nil(t, err, fmt.Sprintf("error verifying bin %d", b))
			assert.True(t, ok, fmt.Sprintf("invalid data at bin %d", b))
		}
		return dst
	}

	src := v0
	for i := 0; i < 3; i++ {
		t.Run(fmt.Sprintf("gen: %d", i), func(t *testing.T) {
			src = verify(t, src)
		})
	}
}
