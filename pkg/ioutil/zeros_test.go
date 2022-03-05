package ioutil

import (
	"bytes"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestWriteZerosN(t *testing.T) {
	var b bytes.Buffer

	cases := []struct {
		n int64
	}{
		{0},
		{11},
		{222},
		{3333},
		{44444},
		{5555555},
		{32 * 1024},
		{64 * 1024},
	}
	for _, c := range cases {
		b.Reset()
		WriteZerosN(&b, c.n)
		assert.EqualValues(t, c.n, b.Len())
	}
}
