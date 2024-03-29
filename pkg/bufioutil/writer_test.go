// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bufioutil

import (
	"bytes"
	"errors"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestWriterPassthrough(t *testing.T) {
	var b bytes.Buffer
	w := NewWriter(&b, 128)
	n, err := w.Write(make([]byte, 128))
	assert.Equal(t, 128, n, "write size mismatch")
	assert.NoError(t, err, "write should not return error")
	assert.Equal(t, 128, b.Len(), "output size mismatch")
	assert.Equal(t, 0, w.n, "writer should have 0 buffered bytes")
}

func TestWriterBuffered(t *testing.T) {
	var b bytes.Buffer
	w := NewWriter(&b, 128)
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

func TestWriterFlushEmpty(t *testing.T) {
	w := NewWriter(&failWriter{}, 128)
	err := w.Flush()
	assert.NoError(t, err)
}

func TestWriterFailure(t *testing.T) {
	w := NewWriter(&failWriter{}, 128)
	w.Write(make([]byte, 50))
	_, err := w.Write(make([]byte, 100))
	assert.ErrorIs(t, errTest, err)

	w = NewWriter(&failWriter{}, 128)
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
