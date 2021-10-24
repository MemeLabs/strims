package apptest

import (
	"io"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestTransfer(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.NoError(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.NoError(t, err)

	key, err := dao.GenerateKey()
	assert.NoError(t, err)

	done := make(chan struct{})

	go func() {
		s, err := ppspp.NewSwarm(key.Public, ppspp.NewDefaultSwarmOptions())
		assert.NoError(t, err)

		tid := ctrl[1].Transfer().Add(s, []byte{})
		ctrl[1].Transfer().Publish(tid, networkKey)

		var limit int64 = 1024 * 1024
		n, err := io.Copy(io.Discard, io.LimitReader(s.Reader(), limit))
		assert.Equal(t, limit, n)
		assert.NoError(t, err)

		close(done)
	}()

	time.Sleep(100 * time.Millisecond)

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.NewDefaultSwarmOptions(),
		Key:          key,
	})
	assert.NoError(t, err)

	tid := ctrl[0].Transfer().Add(w.Swarm(), []byte{})
	ctrl[0].Transfer().Publish(tid, networkKey)

	b := make([]byte, 128*1024)
	writeTicker := time.NewTicker(100 * time.Millisecond)
WriteLoop:
	for {
		select {
		case <-writeTicker.C:
			w.Write(b)
		case <-done:
			break WriteLoop
		}
	}
}
