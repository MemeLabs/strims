package kademlia

import (
	"testing"
)

func TestXOR(t *testing.T) {
	a := ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff}
	b := ID{0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff, 0x0000ffff0000ffff}
	c := ID{0xffff0000ffff0000, 0xffff0000ffff0000, 0xffff0000ffff0000, 0xffff0000ffff0000}
	if !a.XOr(b).Equals(c) {
		t.Fail()
	}
}
