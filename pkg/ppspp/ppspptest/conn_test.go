package ppspptest

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestConn(t *testing.T) {
	a, b := NewConnPair()
	d := make([]byte, 500)

	n, err := a.Write(d)
	assert.Equal(t, len(d), n, "write length mismatch")
	assert.NoError(t, err, "write error")

	err = a.Flush()
	assert.NoError(t, err, "flush error")

	rd := make([]byte, 1000)
	n, err = b.Read(rd)
	assert.Equal(t, len(d), n, "write length mismatch")
	assert.NoError(t, err, "write error")
}
