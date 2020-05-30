package kademlia

import (
	"testing"
)

func TestXOR(t *testing.T) {
	a := ID{0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff}
	b := ID{0x0000ffff, 0x0000ffff, 0x0000ffff, 0x0000ffff, 0x0000ffff}
	c := ID{0xffff0000, 0xffff0000, 0xffff0000, 0xffff0000, 0xffff0000}
	if !a.XOr(b).Equals(c) {
		t.Fail()
	}
}
