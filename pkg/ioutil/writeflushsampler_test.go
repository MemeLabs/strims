package ioutil

import (
	"bytes"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/bufioutil"
	"github.com/stretchr/testify/assert"
)

func TestWriteFlushSampler(t *testing.T) {
	var b bytes.Buffer
	w := NewWriteFlushSampler(bufioutil.NewWriter(&b, 128))

	n, err := w.Write(make([]byte, 128))
	assert.NoError(t, err)
	assert.EqualValues(t, 128, n)
	err = w.Flush()
	assert.NoError(t, err)

	done := make(chan struct{})
	go func() {
		var buf bytes.Buffer
		err := w.Sample(&buf)
		assert.NoError(t, err)
		assert.EqualValues(t, 64, buf.Len())
		close(done)
	}()

	time.Sleep(time.Millisecond)

	n, err = w.Write(make([]byte, 64))
	assert.NoError(t, err)
	assert.EqualValues(t, 64, n)
	err = w.Flush()
	assert.NoError(t, err)

	<-done
}
