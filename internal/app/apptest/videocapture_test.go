package apptest

import (
	"context"
	"crypto/rand"
	"io"
	"io/ioutil"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestVideoCapture(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.NoError(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.NoError(t, err)

	done := make(chan struct{})

	key, err := dao.GenerateKey()
	assert.NoError(t, err)

	options := ppspp.WriterOptions{
		Key:          key,
		SwarmOptions: ppspp.NewDefaultSwarmOptions(),
	}

	go func() {
		swarmURI := ppspp.NewURI(key.Public, options.SwarmOptions.URIOptions()).String()
		_, r, err := ctrl[0].VideoEgress().OpenStream(context.Background(), swarmURI, [][]byte{networkKey})
		assert.NoError(t, err)
		assert.NotNil(t, r)

		var n int64
		for n < 1024*1024 {
			nn, err := io.Copy(ioutil.Discard, r)
			if err != nil && err != io.EOF {
				assert.NoError(t, err)
			}
			n += nn
		}

		r.Close()
		close(done)
	}()

	time.Sleep(100 * time.Millisecond)

	id, err := ctrl[1].VideoCapture().OpenWithSwarmWriterOptions(
		"application/binary+noise",
		&networkv1directory.ListingSnippet{},
		[][]byte{networkKey},
		options,
	)
	assert.NoError(t, err)

	var b [16 * 1024]byte
	_, err = rand.Read(b[:])
	assert.NoError(t, err)

	var n int
	writeTicker := time.NewTicker(100 * time.Millisecond)
WriteLoop:
	for {
		select {
		case <-writeTicker.C:
			n += len(b)
			segmentEnd := n >= 256*1024
			if segmentEnd {
				n = 0
			}
			err = ctrl[1].VideoCapture().Append(id, b[:], segmentEnd)
			assert.NoError(t, err)
		case <-done:
			break WriteLoop
		}
	}

	ctrl[1].VideoCapture().Close(id)
}
