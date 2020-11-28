package kademlia

import (
	"crypto/sha256"
	"testing"

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
