package merkle

import (
	"crypto/sha256"
	"io"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/hmac_drbg"
	"github.com/stretchr/testify/assert"
)

func rng() io.Reader {
	return hmac_drbg.NewReader(sha256.New, []byte{1, 2, 3, 4})
}

func TestVerify(t *testing.T) {
	const chunkSize = 1024
	bin := binmap.NewBin(5, 0)
	data := make([]byte, bin.BaseLength()*chunkSize)
	if _, err := io.ReadFull(rng(), data); err != nil {
		t.Fatal(err)
	}

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	r0 := NewTree(bin, chunkSize, sha256.New())
	r0.SetRoot(r.Get(bin))

	r1 := NewProvisionalTree(r0)
	_, verified := r1.Verify(bin, data)
	assert.True(t, verified, "expected successful validation")

	r0.Merge(r1)
}

func TestVerifyMerge(t *testing.T) {
	const chunkSize = 1024
	bin := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, bin.BaseLength()*chunkSize)
	if _, err := io.ReadFull(rng(), data); err != nil {
		t.Fatal(err)
	}
	fillBytes := data[fillBin.BaseOffset()*chunkSize : (fillBin.BaseOffset()+fillBin.BaseLength())*chunkSize]

	r := NewTree(bin, chunkSize, sha256.New())
	r.Fill(bin, data)

	copyAndVerify := func(dst, src *Tree) bool {
		for b := fillBin; b != bin; b = b.Parent() {
			dst.Set(b.Sibling(), src.Get(b.Sibling()))
		}

		_, verified := dst.Verify(fillBin, fillBytes)
		return verified
	}

	r00 := NewTree(bin, chunkSize, sha256.New())
	r00.SetRoot(r.Get(bin))
	r01 := NewProvisionalTree(r00)
	assert.True(t, copyAndVerify(r01, r), "expected successful validation")

	r00.Merge(r01)

	r10 := NewTree(bin, chunkSize, sha256.New())
	r10.SetRoot(r10.Get(bin))
	r11 := NewProvisionalTree(r10)
	assert.True(t, copyAndVerify(r11, r00), "expected successful validation")
}

func TestVerifyForward(t *testing.T) {
	const chunkSize = 1024
	bin := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, bin.BaseLength()*chunkSize)
	if _, err := io.ReadFull(rng(), data); err != nil {
		t.Fatal(err)
	}

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

	_, verified := r1.Verify(17, data[8*chunkSize:10*chunkSize])
	assert.True(t, verified)
}

func TestNoVeriefiedReferenceNode(t *testing.T) {
	const chunkSize = 1024
	bin := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, bin.BaseLength()*chunkSize)
	if _, err := io.ReadFull(rng(), data); err != nil {
		t.Fatal(err)
	}

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
	_, verified := r1.Verify(17, data[8*chunkSize:10*chunkSize])
	assert.False(t, verified)
}
