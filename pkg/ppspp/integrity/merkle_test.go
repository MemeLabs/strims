package integrity

import (
	"crypto/sha256"
	"io"
	"os"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/stretchr/testify/assert"
)

type discardWriter struct{}

func (d *discardWriter) Write(p []byte) (int, error) {
	return len(p), nil
}

func (d *discardWriter) Flush() error {
	return nil
}

func TestWriter(t *testing.T) {
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
