package queue

// Ring ...
type Ring struct {
	size uint64
	mask uint64
	low  uint64
	high uint64
}

// Size ...
func (r *Ring) Size() uint64 {
	return r.size
}

// Resize ...
func (r *Ring) Resize(size uint64) {
	if size <= r.size {
		return
	}

	r.size = size
	r.mask = size - 1

	if r.size&r.mask != 0 {
		panic("ring size should be power of 2")
	}
	return
}

// Head ...
func (r *Ring) Head() (i uint64, ok bool) {
	i = r.high
	if r.low < i {
		return i & r.mask, true
	}
	return
}

// Tail ...
func (r *Ring) Tail() (i uint64, ok bool) {
	i = r.low
	if i < r.high {
		return i & r.mask, true
	}
	return
}

// Push ...
func (r *Ring) Push() (i uint64, ok bool) {
	i = r.high
	if r.low+r.size > i {
		r.high++
		return i & r.mask, true
	}
	return
}

// Pop ...
func (r *Ring) Pop() (i uint64, ok bool) {
	i = r.low
	if i < r.high {
		r.low++
		return i & r.mask, true
	}
	return
}
