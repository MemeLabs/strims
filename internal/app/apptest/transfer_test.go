package apptest

import (
	"crypto/rand"
	"sync/atomic"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestTransfer(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.NoError(t, err)

	key, err := dao.GenerateKey()
	assert.Nil(t, err)

	swarmID := ppspp.NewSwarmID(key.Public)
	swarmOpt := ppspp.SwarmOptions{
		ChunkSize:          1024,
		LiveWindow:         8 * 1024,
		ChunksPerSignature: 32,
		Integrity: integrity.VerifierOptions{
			ProtectionMethod: integrity.ProtectionMethodMerkleTree,
		},
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: swarmOpt,
		Key:          key,
	})
	assert.Nil(t, err)

	time.Sleep(100 * time.Millisecond)

	go func() {
		b := make([]byte, 128*1024)
		_, err := rand.Read(b)
		assert.Nil(t, err)

		cw, err := chunkstream.NewWriter(w)
		assert.Nil(t, err)
		for range time.NewTicker(100 * time.Millisecond).C {
			cw.Write(b)
			cw.Flush()
		}
	}()

	tid0 := ctrl[0].Transfer().Add(w.Swarm(), []byte{})
	ctrl[0].Transfer().Publish(tid0, networkKey)

	s, err := ppspp.NewSwarm(swarmID, swarmOpt)
	assert.Nil(t, err)

	var total uint64
	done := make(chan struct{})

	go func() {
		b := make([]byte, 16*1024)
		for {
			n, err := s.Reader().Read(b)
			if err != nil {
				panic(err)
			}
			if atomic.AddUint64(&total, uint64(n)) >= 1024*1024 {
				close(done)
				return
			}
		}
	}()

	tid1 := ctrl[1].Transfer().Add(s, []byte{})
	ctrl[1].Transfer().Publish(tid1, networkKey)

	<-done
}
