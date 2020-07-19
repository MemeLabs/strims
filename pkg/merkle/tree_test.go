package merkle

import (
	"crypto/rand"
	"crypto/sha256"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/davecgh/go-spew/spew"
	"github.com/stretchr/testify/assert"
)

// should verify that verifying chunks in different orders works... like forward and backward and in chunks and striped...
// should test that it fails with incorrect data

func TestVerify(t *testing.T) {
	chunkSize := 1024
	bin := binmap.NewBin(5, 0)
	data := make([]byte, int(bin.BaseLength())*chunkSize)
	rand.Read(data)

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	r0 := NewTree(bin, chunkSize, sha256.New())
	r0.SetRoot(r.Get(bin))

	r1 := NewProvisionalTree(r0)
	assert.True(t, r1.Verify(bin, data), "expected successful validation")

	r0.Merge(r1)
	spew.Dump(r0)
}

func TestVerifyForward(t *testing.T) {
	chunkSize := 1024
	bin := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, int(bin.BaseLength())*chunkSize)
	rand.Read(data)

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	// create reference node with the root hash set
	r0 := NewTree(bin, chunkSize, sha256.New())
	r0.SetRoot(r.Get(bin))

	r1 := NewProvisionalTree(r0)
	// set hashes required to verify node on r1
	for b := fillBin; b != bin; b = b.Parent() {
		r1.Set(b.Sibling(), r.Get(b.Sibling()))
	}

	verified := r1.Verify(17, data[8*chunkSize:10*chunkSize])

	assert.True(t, verified)
	spew.Dump(r)
}

func TestNoVeriefiedReferenceNode(t *testing.T) {
	chunkSize := 1024
	bin := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, int(bin.BaseLength())*chunkSize)
	rand.Read(data)

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	// create reference node with no root hash
	r0 := NewTree(bin, chunkSize, sha256.New())

	r1 := NewProvisionalTree(r0)
	// set hashes required to verify node on r1
	for b := fillBin; b != bin; b = b.Parent() {
		r1.Set(b.Sibling(), r.Get(b.Sibling()))
	}

	// should return false seince r0 has no hashes to verify against
	verified := r1.Verify(17, data[8*chunkSize:10*chunkSize])

	assert.False(t, verified)
	spew.Dump(r)
}
