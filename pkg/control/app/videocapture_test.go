package app

import (
	"crypto/rand"
	"io"
	"io/ioutil"
	"testing"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/stretchr/testify/assert"

	"go.uber.org/zap"
)

func TestVideoCapture(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	networkKey, ctrl, err := NewTestControlPair(logger)
	assert.Nil(t, err)

	done := make(chan struct{})

	key, err := dao.GenerateKey()
	assert.Nil(t, err)

	options := ppspp.WriterOptions{
		Key:          key,
		SwarmOptions: ppspp.NewDefaultSwarmOptions(),
	}

	go func() {
		swarmURI := ppspp.NewURI(key.Public, options.SwarmOptions.URIOptions()).String()
		_, r, err := ctrl[0].VideoEgress().OpenStream(swarmURI, [][]byte{networkKey})
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

	id, err := ctrl[1].VideoCapture().OpenWithSwarmWriterOptions(
		"application/binary+noise",
		&networkv1.DirectoryListingSnippet{},
		[][]byte{networkKey},
		options,
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