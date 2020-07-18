package merkle

import (
	"crypto/sha256"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/davecgh/go-spew/spew"
	"github.com/stretchr/testify/assert"
)

func TestVerify(t *testing.T) {
	chunkSize := 1024
	bin := binmap.NewBin(5, 0)
	data := make([]byte, int(bin.BaseLength())*chunkSize)

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	r0 := NewTree(bin, chunkSize, sha256.New())
	r0.SetRoot(r.Get(bin))

	r1 := NewProvisionalTree(r0)
	assert.True(t, r1.Verify(bin, data), "expected successful validation")

	r0.Merge(r1)
	spew.Dump(r0)
}
