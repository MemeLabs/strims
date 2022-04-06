package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/slab"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type timeSet struct {
	root      *timeSetNode
	allocator *slab.Allocator[timeSetNode]
}

func (t *timeSet) Size() int {
	return t.root.size()
}

func (t *timeSet) Set(bin binmap.Bin, time timeutil.Time) timeutil.Time {
	if t.root == nil {
		t.allocator = slab.New[timeSetNode]()
		t.root = &timeSetNode{bin: bin}
	}

	for !t.root.bin.Contains(bin) {
		r := &timeSetNode{bin: t.root.bin.Parent(), count: t.root.count}

		if r.bin < t.root.bin {
			r.right = t.root
		} else {
			r.left = t.root
		}
		t.root = r
	}

	return t.root.set(bin, time).time
}

func (t *timeSet) Unset(bin binmap.Bin) {
	if _, deleted := t.root.unset(bin); deleted {
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
		r.left.delete()
		t.root = r.right
	}
	t.root.prune(bin)
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

func (r *timeSetNode) set(b binmap.Bin, t timeutil.Time) *timeSetNode {
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
			r.right = &timeSetNode{bin: r.bin.Right()}
		} else {
			r.count -= r.right.count
		}
		n = r.right.set(b, t)
		r.count += r.right.count
	} else {
		if r.left == nil {
			r.left = &timeSetNode{bin: r.bin.Left()}
		} else {
			r.count -= r.left.count
		}
		n = r.left.set(b, t)
		r.count += r.left.count
	}
	return n
}

func (r *timeSetNode) unset(b binmap.Bin) (bool, bool) {
	if r == nil {
		return false, false
	}

	var ok, deleted bool
	if r.bin < b {
		ok, deleted = r.right.unset(b)
		if deleted {
			r.right = nil
		}
	} else {
		ok, deleted = r.left.unset(b)
		if deleted {
			r.left = nil
		}
	}

	ok = ok || r.bin.BaseLength() == r.count
	if ok {
		r.count -= b.BaseLength()
		if r.count == 0 {
			r.delete()
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
