package bufwriter

import (
	"bytes"
	"errors"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestWriterPassthrough(t *testing.T) {
	var b bytes.Buffer
	w := New(&b, 128)
	n, err := w.Write(make([]byte, 128))
	assert.Equal(t, 128, n, "write size mismatch")
	assert.NoError(t, err, "write should not return error")
	assert.Equal(t, 128, b.Len(), "output size mismatch")
	assert.Equal(t, 0, w.n, "writer should have 0 buffered bytes")
}

func TestWriterBuffered(t *testing.T) {
	var b bytes.Buffer
	w := New(&b, 128)
	n, err := w.Write(make([]byte, 50))
	assert.Equal(t, 50, n)
	assert.NoError(t, err)
	assert.Equal(t, 0, b.Len())
	assert.Equal(t, 50, w.n)

	n, err = w.Write(make([]byte, 100))
	assert.Equal(t, 100, n)
	assert.NoError(t, err)
	assert.Equal(t, 128, b.Len())
	assert.Equal(t, 22, w.n)
}

func TestWriterFailure(t *testing.T) {
	w := New(&failWriter{}, 128)
	w.Write(make([]byte, 50))
	_, err := w.Write(make([]byte, 100))
	assert.ErrorIs(t, errTest, err)

	w = New(&failWriter{}, 128)
	w.Write(make([]byte, 128))
	assert.ErrorIs(t, errTest, err)
}

var errTest = errors.New("failed")

type failWriter struct{}

func (w *failWriter) Write(p []byte) (int, error) {
	return 0, errTest
}

func (w *failWriter) Flush() error {
	return errTest
}
