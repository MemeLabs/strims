package app

import (
	"context"
	"crypto/rand"
	"io"
	"io/ioutil"
	"testing"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestVideoCapture(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.Nil(t, err)

	done := make(chan struct{})

	go func() {
		events := ctrl[0].Directory().ReadEvents(context.Background(), networkKey)
		var m *networkv1.DirectoryListingMedia
		for e := range events {
			if m = e.GetPublish().GetListing().GetMedia(); m != nil {
				break
			}
		}

		logger.Debug("received swarm uri", zap.String("uri", m.SwarmUri))

		_, r, err := ctrl[0].VideoEgress().OpenStream(m.SwarmUri, [][]byte{networkKey})
		assert.Nil(t, err)

		var n int64
		for n < 1024*1024 {
			nn, err := io.Copy(ioutil.Discard, r)
			if err != nil && err != io.EOF {
				assert.Nil(t, err)
			}
			n += nn
		}

		r.Close()
		close(done)
	}()

	time.Sleep(time.Second)

	id, err := ctrl[1].VideoCapture().Open(
		"application/binary+noise",
		&networkv1.DirectoryListingSnippet{},
		[][]byte{networkKey},
	)
	assert.Nil(t, err)

	var b [16 * 1024]byte
	_, err = rand.Read(b[:])
	assert.Nil(t, err)

	writeTicker := time.NewTicker(time.Millisecond * 10)

	var n int
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
			assert.Nil(t, err)
		case <-done:
			break WriteLoop
		}
	}

	ctrl[1].VideoCapture().Close(id)
}
