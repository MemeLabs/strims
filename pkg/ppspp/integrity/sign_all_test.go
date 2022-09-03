// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	"bytes"
	"crypto/rand"
	"fmt"
	"io"
	"testing"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/blake2b"
)

type testWriter struct {
	SignedIntegrity codec.SignedIntegrity
	Integrity       []codec.Integrity
}

func (w *testWriter) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	w.SignedIntegrity = m
	return 0, nil
}

func (w *testWriter) WriteIntegrity(m codec.Integrity) (int, error) {
	w.Integrity = append(w.Integrity, m)
	return 0, nil
}

func TestSignAllVerifier(t *testing.T) {
	key, err := dao.GenerateKey()
	assert.Nil(t, err)

	const chunkSize = 1024
	const liveDiscardWindow = 1024
	const n = 1 << 20

	v0 := NewSignAllSwarmVerifier(&SignAllOptions{
		LiveDiscardWindow: liveDiscardWindow,
		ChunkSize:         chunkSize,
		Verifier:          NewED25519Verifier(key.Public),
		Hash:              blake2bFunc(blake2b.New256),
	})

	var b bytes.Buffer
	w := NewSignAllWriter(&SignAllWriterOptions{
		Verifier:  v0,
		Writer:    bufioutil.NewWriter(&b, 1024),
		ChunkSize: chunkSize,
		Signer:    NewED25519Signer(key.Private),
	})
	io.CopyN(w, rand.Reader, n)
	d := b.Bytes()

	verify := func(t *testing.T, src *SignAllSwarmVerifier) *SignAllSwarmVerifier {
		dst := NewSignAllSwarmVerifier(&SignAllOptions{
			LiveDiscardWindow: liveDiscardWindow,
			ChunkSize:         chunkSize,
			Verifier:          NewED25519Verifier(key.Public),
			Hash:              blake2bFunc(blake2b.New256),
		})

		cv := dst.ChannelVerifier()
		for i := 0; i < n/chunkSize; i++ {
			b := binmap.Bin(i * 2)
			v := cv.ChunkVerifier(b)

			var w testWriter
			_, err := src.WriteIntegrity(b, binmap.New(), &w)
			assert.Nil(t, err, fmt.Sprintf("failed to write integrity mesages bin %d", b))

			v.SetSignedIntegrity(b, w.SignedIntegrity.Timestamp.Time, w.SignedIntegrity.Signature)
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
