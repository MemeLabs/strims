package merkle

import (
	"crypto/sha256"
	"fmt"
	"io"
	"math/rand"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/hmac_drbg"
	"github.com/stretchr/testify/assert"
)

func rng() io.Reader {
	return hmac_drbg.NewReader(sha256.New, []byte{1, 2, 3, 4})
}

func TestVerify(t *testing.T) {
	cases := []struct {
		chunkSize int
		bin       binmap.Bin
	}{
		{
			chunkSize: 1024,
			bin:       binmap.NewBin(5, 0),
		},
		{
			chunkSize: 4096,
			bin:       binmap.NewBin(6, 0),
		},
	}

	for _, c := range cases {
		c := c
		t.Run(fmt.Sprintf("chunkSize: %d, rootBin: %d", c.chunkSize, c.bin), func(t *testing.T) {
			data := make([]byte, c.bin.BaseLength()*uint64(c.chunkSize))
			if _, err := io.ReadFull(rng(), data); err != nil {
				t.Fatal(err)
			}

			r := NewTree(c.bin, c.chunkSize, sha256.New())
			r.Fill(c.bin, data)

			// spew.Dump(r)

			r0 := NewTree(c.bin, c.chunkSize, sha256.New())
			r0.SetRoot(r.Get(c.bin))

			r1 := NewProvisionalTree(r0)
			verified, err := r1.Verify(c.bin, data)
			assert.Nil(t, err, "unexpected error")
			assert.True(t, verified, "expected successful validation")

			r0.Merge(r1)
		})
	}
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

		verified, err := dst.Verify(fillBin, fillBytes)
		assert.Nil(t, err, "unexpected error")

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

func BenchmarkMerge(b *testing.B) {
	const chunkSize = 1024
	root := binmap.NewBin(6, 0)

	newTestTree := func(b binmap.Bin) *Tree {
		t := NewTree(root, chunkSize, sha256.New())

		for i := b.BaseLeft(); i < b.BaseRight(); i++ {
			t.setVerified(i)
		}

		for b = b.Parent(); b != root; b = b.Parent() {
			t.setVerified(b)
			t.setVerified(b.Sibling())
		}

		return t
	}

	var trees []*Tree
	for l := uint64(0); l <= 6; l++ {
		for o := uint64(0); binmap.NewBin(l, o).BaseRight() < root.BaseRight(); o++ {
			trees = append(trees, newTestTree(binmap.NewBin(l, o)))
		}
	}

	rand.Seed(1234)

	b.ResetTimer()

	t := NewTree(root, chunkSize, sha256.New())
	for i := 0; i < b.N; i++ {
		t.verified = trees[rand.Intn(len(trees))].verified
		t.Merge(trees[rand.Intn(len(trees))])
	}
}

func TestVerifyForward(t *testing.T) {
	const chunkSize = 1024
	root := binmap.NewBin(5, 0)
	fillBin := binmap.NewBin(1, 4)
	data := make([]byte, root.BaseLength()*chunkSize)
	if _, err := io.ReadFull(rng(), data); err != nil {
		t.Fatal(err)
	}

	r := NewTree(root, chunkSize, sha256.New())
	r.Fill(root, data)

	// create reference node with the root hash set
	r0 := NewTree(root, chunkSize, sha256.New())
	r0.SetRoot(r.Get(root))

	r1 := NewProvisionalTree(r0)
	// set hashes required to verify node on r1
	for b := fillBin; b != root; b = b.Parent() {
		r1.Set(b.Sibling(), r.Get(b.Sibling()))
	}

	verified, err := r1.Verify(17, data[8*chunkSize:10*chunkSize])
	assert.Nil(t, err, "unexpected error")
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

	// should return false since r0 has no hashes to verify against
	verified, err := r1.Verify(17, data[8*chunkSize:10*chunkSize])
	assert.Nil(t, err, "unexpected error")
	assert.False(t, verified)
}
