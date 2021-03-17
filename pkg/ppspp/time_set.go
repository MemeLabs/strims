package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

type timeSet struct {
	root *timeSetNode
}

func (t *timeSet) Set(bin binmap.Bin, time int64) int64 {
	if t.root == nil {
		t.root = newTimeSetNode(bin)
	}

	for !t.root.bin.Contains(bin) {
		r := newTimeSetNodeWithCount(t.root.bin.Parent(), t.root.count)
		if r.bin < t.root.bin {
			r.right = t.root
		} else {
			r.left = t.root
		}
		t.root = r
	}

	return t.root.set(bin, time).time
}

func (t *timeSet) Get(bin binmap.Bin) (int64, bool) {
	n := t.root.get(bin)
	if n == nil {
		return 0, false
	}
	return n.time, true
}

func (t *timeSet) Prune(bin binmap.Bin) {
	for t.root != nil && t.root.bin < bin {
		r := t.root
		r.left.delete()
		t.root = r.right
	}
	t.root.prune(bin)
}

func newTimeSetNode(b binmap.Bin) *timeSetNode {
	return newTimeSetNodeWithCount(b, 0)
}

func newTimeSetNodeWithCount(b binmap.Bin, c uint64) *timeSetNode {
	return &timeSetNode{
		bin:   b,
		count: c,
	}
}

type timeSetNode struct {
	left  *timeSetNode
	right *timeSetNode
	bin   binmap.Bin
	count uint64
	time  int64
}

func (r *timeSetNode) set(b binmap.Bin, t int64) *timeSetNode {
	if r.bin == b {
		if r.time == 0 {
			r.count = r.bin.BaseLength()
			r.time = t
		}
		return r
	}

	if r.bin.BaseLength() == r.count {
		return r.get(b)
	}

	var n *timeSetNode
	if r.bin < b {
		if r.right == nil {
			r.right = newTimeSetNode(r.bin.Right())
		} else {
			r.count -= r.right.count
		}
		n = r.right.set(b, t)
		r.count += r.right.count
	} else {
		if r.left == nil {
			r.left = newTimeSetNode(r.bin.Left())
		} else {
			r.count -= r.left.count
		}
		n = r.left.set(b, t)
		r.count += r.left.count
	}
	return n
}

func (r *timeSetNode) get(b binmap.Bin) *timeSetNode {
	if r == nil {
		return nil
	}

	var n *timeSetNode
	if r.bin < b {
		n = r.right.get(b)
	} else {
		n = r.left.get(b)
	}

	if n == nil && r.bin.BaseLength() == r.count {
		return r
	}
	return n
}

func (r *timeSetNode) prune(b binmap.Bin) {
	if r == nil {
		return
	}

	if r.bin < b {
		r.left.delete()
		r.left = nil
		r.right.prune(b)
	} else {
		r.left.prune(b)
	}
}

func (r *timeSetNode) delete() {
	if r == nil {
		return
	}

	r.left.delete()
	r.right.delete()
}
