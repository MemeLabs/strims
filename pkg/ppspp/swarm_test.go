package ppspp

import (
	"math/rand"
	"testing"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/stretchr/testify/assert"
)

func TestCache(t *testing.T) {
	key, err := dao.GenerateKey()
	assert.NoError(t, err)

	opt := NewDefaultSwarmOptions()

	w, err := NewWriter(WriterOptions{
		SwarmOptions: opt,
		Key:          key,
	})

	b := make([]byte, opt.ChunkSize*opt.ChunksPerSignature*8)
	_, err = rand.Read(b)
	assert.NoError(t, err)

	_, err = w.Write(b)
	assert.NoError(t, err)

	c, err := w.Swarm().ExportCache()
	assert.NoError(t, err)

	// import
	s := NewDefaultSwarm(key.Public)
	s.ImportCache(c)

	b2 := make([]byte, len(b))
	_, err = s.Reader().Read(b2)
	assert.NoError(t, err)

	assert.EqualValues(t, b, b2)
}
