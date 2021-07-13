package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type timeSet struct {
	root     *timeSetNode
	freeHead *timeSetNode
}

func (t *timeSet) Size() int {
	return t.root.size()
}

func (t *timeSet) Set(bin binmap.Bin, time timeutil.Time) timeutil.Time {
	if t.root == nil {
		t.root = t.allocNode(bin, 0)
	}

	for !t.root.bin.Contains(bin) {
		r := t.allocNode(t.root.bin.Parent(), t.root.count)

		if r.bin < t.root.bin {
			r.right = t.root
		} else {
			r.left = t.root
		}
		t.root = r
	}

	return t.root.set(bin, time, t).time
}

func (t *timeSet) Unset(bin binmap.Bin) {
	if _, deleted := t.root.unset(bin, t); deleted {
		t.root = nil
	}
}

func (t *timeSet) Get(bin binmap.Bin) (binmap.Bin, timeutil.Time, bool) {
	n := t.root.get(bin)
	if n == nil || n.time == 0 {
		return 0, 0, false
	}
	return n.bin, n.time, true
}

func (t *timeSet) Prune(bin binmap.Bin) {
	for t.root != nil && t.root.bin < bin {
		r := t.root
		r.left.delete(t)
		t.root = r.right
	}
	t.root.prune(bin, t)
}

func (t *timeSet) allocNode(b binmap.Bin, c uint64) *timeSetNode {
	r := t.freeHead
	if r == nil {
		r = &timeSetNode{}
	} else {
		t.freeHead = r.right
		r.right = nil
	}

	r.bin = b
	r.count = c
	return r
}

func (t *timeSet) freeNode(r *timeSetNode) {
	r.left = nil
	r.right = t.freeHead
	r.time = 0
	t.freeHead = r
}

type timeSetNodeAllocator interface {
	allocNode(b binmap.Bin, c uint64) *timeSetNode
	freeNode(r *timeSetNode)
}

type timeSetNode struct {
	left  *timeSetNode
	right *timeSetNode
	bin   binmap.Bin
	count uint64
	time  timeutil.Time
}

func (r *timeSetNode) size() int {
	if r == nil {
		return 0
	}
	return r.left.size() + r.right.size() + 1
}

func (r *timeSetNode) set(b binmap.Bin, t timeutil.Time, a timeSetNodeAllocator) *timeSetNode {
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
			r.right = a.allocNode(r.bin.Right(), 0)
		} else {
			r.count -= r.right.count
		}
		n = r.right.set(b, t, a)
		r.count += r.right.count
	} else {
		if r.left == nil {
			r.left = a.allocNode(r.bin.Left(), 0)
		} else {
			r.count -= r.left.count
		}
		n = r.left.set(b, t, a)
		r.count += r.left.count
	}
	return n
}

func (r *timeSetNode) unset(b binmap.Bin, a timeSetNodeAllocator) (bool, bool) {
	if r == nil {
		return false, false
	}

	var ok, deleted bool
	if r.bin < b {
		ok, deleted = r.right.unset(b, a)
		if deleted {
			r.right = nil
		}
	} else {
		ok, deleted = r.left.unset(b, a)
		if deleted {
			r.left = nil
		}
	}

	ok = ok || r.bin.BaseLength() == r.count
	if ok {
		r.count -= b.BaseLength()
		if r.count == 0 {
			r.delete(a)
			return true, true
		}
	}
	return ok, false
}

func (r *timeSetNode) get(b binmap.Bin) *timeSetNode {
	if r == nil || !r.bin.Contains(b) {
		return nil
	}
	if r.bin == b {
		return r
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

func (r *timeSetNode) prune(b binmap.Bin, a timeSetNodeAllocator) {
	if r == nil {
		return
	}

	if r.bin < b {
		r.left.delete(a)
		r.left = nil
		r.right.prune(b, a)
	} else {
		r.left.prune(b, a)
	}
}

func (r *timeSetNode) delete(a timeSetNodeAllocator) {
	if r == nil {
		return
	}

	r.left.delete(a)
	r.right.delete(a)

	a.freeNode(r)
}
