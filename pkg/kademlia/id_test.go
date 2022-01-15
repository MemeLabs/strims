package kademlia

import (
	"crypto/sha256"
	"testing"

	"github.com/davecgh/go-spew/spew"
	"github.com/stretchr/testify/assert"
)

func TestXOR(t *testing.T) {
	a := ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff}
	b := ID{0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff}
	c := ID{0xffff0000ffff0000, 0xffff0000ffff0000, 0xffff0000ffff0000, 0xffff0000ffff0000}
	if !a.XOr(b).Equals(c) {
		t.Fail()
	}
}

func TestMarshalUnmarshal(t *testing.T) {
	hash := sha256.New()
	hash.Write([]byte("test"))
	b0 := hash.Sum(nil)

	id0, err := UnmarshalID(b0)
	assert.Nil(t, err)

	b1 := id0.Bytes(nil)
	id1, err := UnmarshalID(b1)
	assert.Nil(t, err)

	assert.Equal(t, id0, id1)
	assert.Equal(t, b0, b1)
}

func TestBinary(t *testing.T) {
	id := ID{0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff}
	spew.Dump(id.Binary())
	spew.Dump(id.Bytes(nil))
}

var BenchmarkBinaryResult []byte

func BenchmarkBinary(b *testing.B) {
	id := ID{0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff}

	for i := 0; i < b.N; i++ {
		BenchmarkBinaryResult = id.Binary()
	}
}

func BenchmarkBytes(b *testing.B) {
	id := ID{0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff}
	BenchmarkBinaryResult = make([]byte, 32)

	for i := 0; i < b.N; i++ {
		id.Bytes(BenchmarkBinaryResult)
	}
}
