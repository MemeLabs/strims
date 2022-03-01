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

func initRootHash(t *Tree, digest []byte) {
	t.Set(t.rootBin, digest)
	t.setVerified(t.rootBin)
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

			r := NewTree(c.bin, c.chunkSize, sha256.New)
			r.Fill(c.bin, data)

			r0 := NewTree(c.bin, c.chunkSize, sha256.New)
			initRootHash(r0, r.Get(c.bin))

			r1 := NewTree(c.bin, c.chunkSize, sha256.New)
			verified, err := r1.Verify(c.bin, data, r0)
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

	r := NewTree(bin, chunkSize, sha256.New)
	r.Fill(bin, data)

	copyAndVerify := func(dst, src, parent *Tree) bool {
		for b := fillBin; b != bin; b = b.Parent() {
			dst.Set(b.Sibling(), src.Get(b.Sibling()))
		}

		verified, err := dst.Verify(fillBin, fillBytes, parent)
		assert.Nil(t, err, "unexpected error")

		return verified
	}

	vt := NewTree(bin, chunkSize, sha256.New)
	// copy/verify root hash to verified tree (ex. received signed munro)
	initRootHash(vt, r.Get(bin))
	tt0 := NewTree(bin, chunkSize, sha256.New)
	// copy unverified sibling hashes to temp tree (ex. received sibling hashes)
	for b := fillBin; b != bin; b = b.Parent() {
		tt0.Set(b.Sibling(), r.Get(b.Sibling()))
	}
	verified, err := tt0.Verify(fillBin, fillBytes, vt)
	assert.Nil(t, err, "unexpected error")
	assert.True(t, copyAndVerify(tt0, r, vt), "expected successful validation")

	// copy verified hashes from temp to verified tree
	vt.Merge(tt0)

	// confirm all the hashes that verify fillBytes are in the verified tree
	tt1 := NewTree(bin, chunkSize, sha256.New)
	verified, err = tt1.Verify(fillBin, fillBytes, vt)
	assert.Nil(t, err, "unexpected error")
	assert.True(t, verified, "expected successful validation")
}

func BenchmarkMerge(b *testing.B) {
	const chunkSize = 1024
	root := binmap.NewBin(6, 0)

	newTestTree := func(b binmap.Bin) *Tree {
		t := NewTree(root, chunkSize, sha256.New)

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

	t := NewTree(root, chunkSize, sha256.New)
	for i := 0; i < b.N; i++ {
		t.verified = trees[rand.Intn(len(trees))].verified
		t.Merge(trees[rand.Intn(len(trees))])
		t.Reset(root)
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

	r := NewTree(root, chunkSize, sha256.New)
	r.Fill(root, data)

	// create reference node with the root hash set
	r0 := NewTree(root, chunkSize, sha256.New)
	initRootHash(r0, r.Get(root))

	r1 := NewTree(root, chunkSize, sha256.New)
	// set hashes required to verify node on r1
	for b := fillBin; b != root; b = b.Parent() {
		r1.Set(b.Sibling(), r.Get(b.Sibling()))
	}

	verified, err := r1.Verify(17, data[8*chunkSize:10*chunkSize], r0)
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

	r := NewTree(bin, chunkSize, sha256.New)
	r.Fill(bin, data)

	// create reference node with no root hash
	r0 := NewTree(bin, chunkSize, sha256.New)

	r1 := NewTree(bin, chunkSize, sha256.New)
	// set hashes required to verify node on r1
	for b := fillBin; b != bin; b = b.Parent() {
		r1.Set(b.Sibling(), r.Get(b.Sibling()))
	}

	// should return false since r0 has no hashes to verify against
	verified, err := r1.Verify(17, data[8*chunkSize:10*chunkSize], r0)
	assert.Nil(t, err, "unexpected error")
	assert.False(t, verified)
}
