package binmap

func NewEmptyAtIterator(m *Map, b Bin) Iterator {
	return newIterator(b, m.FindFilledAfter, m.FindEmptyAfter)
}

func NewFilledAtIterator(m *Map, b Bin) Iterator {
	return newIterator(b, m.FindEmptyAfter, m.FindFilledAfter)
}

type findFunc func(i Bin) Bin

func newIterator(b Bin, findNeg, findPos findFunc) Iterator {
	i := b.BaseLeft() - 2
	return Iterator{
		findNeg:  findNeg,
		findPos:  findPos,
		i:        i,
		gapRight: i,
		end:      b.BaseRight(),
	}
}

type Iterator struct {
	findNeg  findFunc
	findPos  findFunc
	i        Bin
	end      Bin
	gapLeft  Bin
	gapRight Bin
}

func (e *Iterator) initGap(i Bin) Bin {
	gap := e.findNeg(i)
	if gap == i {
		i = e.findPos(i)
		gap = e.findNeg(i)
	}

	e.gapLeft = gap
	e.gapRight = gap
	return i
}

func (e *Iterator) NextBase() bool {
	return e.NextBaseAfter(e.i + 2)
}

func (e *Iterator) NextBaseAfter(i Bin) bool {
	if i >= e.gapRight+2 {
		i = e.initGap(i)
	} else if i >= e.gapLeft {
		i = e.initGap(e.gapRight + 2)
	}

	e.i = i.BaseLeft()
	return i <= e.end
}

func (e *Iterator) Next() bool {
	i := e.i.LayerRight()
	imin := i.BaseLeft()
	if imin >= e.gapLeft {
		i = e.initGap(e.gapRight + 2)
		imin = i
	}
	return e.nextAfter(i, imin)
}

func (e *Iterator) NextAfter(i Bin) bool {
	imin := i.BaseLeft()
	if i >= e.gapRight+2 {
		i = e.initGap(i)
		imin = i
	} else if i >= e.gapLeft {
		i = e.initGap(e.gapRight + 2)
		imin = i
	}
	return e.nextAfter(i, imin)
}

func (e *Iterator) nextAfter(i, imin Bin) bool {
	if i.IsNone() {
		e.i = i
		return false
	}

	for {
		t := i.Parent()
		if imin > t.BaseLeft() || t.BaseRight() >= e.gapLeft || t.BaseRight() > e.end {
			break
		}
		i = t
	}

	for i.BaseRight() >= e.gapLeft && !i.IsBase() {
		i = i.Left()
	}

	e.i = i
	return i <= e.end
}

func (e *Iterator) Value() Bin {
	return e.i
}

func (e Iterator) ToSlice() []Bin {
	return iteratorToSlice(&e)
}

func (e Iterator) ToBaseSlice() []Bin {
	return baseIteratorToSlice(&e)
}

func (e Iterator) ToSliceAfter(b Bin) []Bin {
	return iteratorAfterToSlice(&e, b)
}

func (e Iterator) ToBaseSliceAfter(b Bin) []Bin {
	return baseIteratorAfterToSlice(&e, b)
}

func NewIntersectionIterator(a, b Iterator) IntersectionIterator {
	return IntersectionIterator{
		it: [2]Iterator{a, b},
		p:  -1,
	}
}

type IntersectionIterator struct {
	it [2]Iterator
	p  int
	i  Bin
}

func (e *IntersectionIterator) NextBase() bool {
	if !e.it[0].NextBase() || !e.it[1].NextBase() {
		return false
	}

	for e.it[0].Value() != e.it[1].Value() {
		for e.it[0].Value() < e.it[1].Value() {
			if !e.it[0].NextBaseAfter(e.it[1].Value()) {
				return false
			}
		}

		for e.it[1].Value() < e.it[0].Value() {
			if !e.it[1].NextBaseAfter(e.it[0].Value()) {
				return false
			}
		}
	}

	e.i = e.it[0].Value()
	return true
}

func (e *IntersectionIterator) Next() bool {
	if e.p == -1 {
		if !e.it[0].Next() {
			return false
		}
		if !e.it[1].NextAfter(e.it[0].Value().BaseLeft()) {
			return false
		}
	} else if !e.it[e.p].Next() {
		return false
	}
	return e.next()
}

func (e *IntersectionIterator) NextAfter(i Bin) bool {
	if !e.it[0].NextAfter(i) {
		return false
	}
	if !e.it[1].NextAfter(e.it[0].Value().BaseLeft()) {
		return false
	}
	return e.next()
}

func (e *IntersectionIterator) next() bool {
	for {
		if e.it[0].Value().BaseRight() < e.it[1].Value().BaseRight() && !e.it[1].Value().Contains(e.it[0].Value()) {
			if !e.it[0].NextAfter(e.it[1].Value().BaseLeft()) {
				return false
			}
		}

		if e.it[1].Value().Contains(e.it[0].Value()) {
			e.i = e.it[0].Value()
			e.p = 0
			return true
		}

		if e.it[1].Value().BaseRight() < e.it[0].Value().BaseRight() && !e.it[0].Value().Contains(e.it[1].Value()) {
			if !e.it[1].NextAfter(e.it[0].Value().BaseLeft()) {
				return false
			}
		}

		if e.it[0].Value().Contains(e.it[1].Value()) {
			e.i = e.it[1].Value()
			e.p = 1
			return true
		}
	}
}

func (e *IntersectionIterator) Value() Bin {
	return e.i
}

func (e IntersectionIterator) ToSlice() []Bin {
	return iteratorToSlice(&e)
}

func (e IntersectionIterator) ToBaseSlice() []Bin {
	return baseIteratorToSlice(&e)
}

func (e IntersectionIterator) ToSliceAfter(b Bin) []Bin {
	return iteratorAfterToSlice(&e, b)
}

type iterator interface {
	Next() bool
	Value() Bin
}

type iteratorAfter interface {
	iterator
	NextAfter(Bin) bool
}

type baseIterator interface {
	NextBase() bool
	Value() Bin
}

type baseIteratorAfter interface {
	baseIterator
	NextBaseAfter(Bin) bool
}

func iteratorToSlice(it iterator) []Bin {
	var bins []Bin
	for it.Next() {
		bins = append(bins, it.Value())
	}
	return bins
}

func baseIteratorToSlice(it baseIterator) []Bin {
	var bins []Bin
	for it.NextBase() {
		bins = append(bins, it.Value())
	}
	return bins
}

func iteratorAfterToSlice(it iteratorAfter, b Bin) []Bin {
	var bins []Bin
	for ok := it.NextAfter(b); ok; ok = it.Next() {
		bins = append(bins, it.Value())
	}
	return bins
}

func baseIteratorAfterToSlice(it baseIteratorAfter, b Bin) []Bin {
	var bins []Bin
	for ok := it.NextBaseAfter(b); ok; ok = it.NextBase() {
		bins = append(bins, it.Value())
	}
	return bins
}
