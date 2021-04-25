package ppspptest

import (
	"bytes"
	"crypto/sha512"
	"fmt"
	"io"
	"sync"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/hmac_drbg"
	"github.com/stretchr/testify/assert"
)

func TestCapLogWriter(t *testing.T) {
	seed := []byte{1, 2, 3, 4}
	writeSize := 4000
	writeCount := 10
	writeThreads := 5

	logBuf := &bytes.Buffer{}
	logWriter := NewCapLogWriter(logBuf)

	var wg sync.WaitGroup
	wg.Add(writeThreads * 4)

	doRead := func(c Conn) {
		io.CopyN(io.Discard, c, int64(writeSize*writeCount))
		wg.Done()
	}
	doWrite := func(c Conn) {
		rng := hmac_drbg.NewReader(sha512.New, seed)
		b := make([]byte, writeSize)
		for j := 0; j < writeCount; j++ {
			_, err := io.ReadFull(rng, b)
			assert.NoError(t, err)
			_, err = c.Write(b)
			assert.NoError(t, err)
			assert.NoError(t, c.Flush())
		}
		wg.Done()
	}

	for i := 0; i < writeThreads; i++ {
		c0, c1 := NewConnPair()

		c0, err := NewCapConn(c0, logWriter.Writer(), fmt.Sprintf("c0_%d", i))
		assert.NoError(t, err)
		c1, err = NewCapConn(c1, logWriter.Writer(), fmt.Sprintf("c1_%d", i))
		assert.NoError(t, err)

		go doRead(c0)
		go doRead(c1)
		go doWrite(c0)
		go doWrite(c1)
	}

	wg.Wait()
	assert.NoError(t, logWriter.Close())

	var hs []*testCapLogHandler
	var mu sync.Mutex
	err := ReadCapLog(logBuf, func() CapLogHandler {
		rng := hmac_drbg.NewReader(sha512.New, seed)
		b := make([]byte, writeSize)
		checkWrite := func(p []byte) {
			_, err := io.ReadFull(rng, b)
			assert.NoError(t, err)
			assert.Equal(t, b, p, "write playback mismatch")
		}

		h := &testCapLogHandler{
			mu:         &mu,
			close:      make(chan struct{}),
			checkWrite: checkWrite,
		}
		hs = append(hs, h)
		return h
	})
	assert.ErrorIs(t, err, io.EOF)

	for _, h := range hs {
		h.Wait()
		assert.Equal(t, uint64(1), h.InitCount, "InitCount mismatch")
		assert.Equal(t, uint64(writeCount), h.WriteCount, "WriteCount mismatch")
		assert.Equal(t, uint64(0), h.WriteErrCount, "WriteErrCount mismatch")
		assert.Equal(t, uint64(writeCount), h.FlushCount, "FlushCount mismatch")
		assert.Equal(t, uint64(0), h.FlushErrCount, "FlushErrCount mismatch")
		assert.Equal(t, uint64(writeCount), h.ReadCount, "ReadCount mismatch")
		assert.Equal(t, uint64(0), h.ReadErrCount, "ReadErrCount mismatch")
	}
}

type testCapLogHandler struct {
	mu         *sync.Mutex
	close      chan struct{}
	checkWrite func(p []byte)

	Label         string
	InitCount     uint64
	WriteCount    uint64
	WriteErrCount uint64
	FlushCount    uint64
	FlushErrCount uint64
	ReadCount     uint64
	ReadErrCount  uint64
}

func (h *testCapLogHandler) Wait() {
	<-h.close
}

func (h *testCapLogHandler) HandleInit(t time.Time, label string) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.Label = label
	h.InitCount++
}

func (h *testCapLogHandler) HandleEOF() {
	close(h.close)
}

func (h *testCapLogHandler) HandleWrite(t time.Time, p []byte) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.WriteCount++
	h.checkWrite(p)
}

func (h *testCapLogHandler) HandleWriteErr(t time.Time, err error) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.WriteErrCount++
}

func (h *testCapLogHandler) HandleFlush(t time.Time) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.FlushCount++
}

func (h *testCapLogHandler) HandleFlushErr(t time.Time, err error) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.FlushErrCount++
}

func (h *testCapLogHandler) HandleRead(t time.Time, p []byte) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.ReadCount++
}

func (h *testCapLogHandler) HandleReadErr(t time.Time, err error) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.ReadErrCount++
}
