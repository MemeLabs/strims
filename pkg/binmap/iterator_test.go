// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package binmap

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestIterateEmptyAtBase(t *testing.T) {
	cases := []struct {
		filled []Bin
		bin    Bin
		label  string
	}{
		{
			filled: []Bin{2, 5, 12, 22, 24},
			label:  "basic",
		},
		{
			filled: []Bin{},
			label:  "none filled",
		},
		{
			filled: []Bin{15},
			label:  "all filled",
		},
		{
			filled: []Bin{7},
			label:  "first half filled",
		},
		{
			filled: []Bin{23},
			label:  "second half filled",
		},
	}

	bins := []Bin{7, 15, 23}

	for _, c := range cases {
		c := c
		for _, b := range bins {
			b := b
			t.Run(fmt.Sprintf("%s, bin %d", c.label, b), func(t *testing.T) {
				k := New()
				for _, i := range c.filled {
					k.Set(i)
				}

				expectedFilled := []Bin{}
				expectedEmpty := []Bin{}

				for i := b.BaseLeft(); i <= b.BaseRight(); i += 2 {
					if k.FilledAt(i) {
						expectedFilled = append(expectedFilled, i)
					} else {
						expectedEmpty = append(expectedEmpty, i)
					}
				}

				actualFilled := []Bin{}
				actualEmpty := []Bin{}

				for it := k.IterateFilledAt(b); it.NextBase(); {
					actualFilled = append(actualFilled, it.Value())
				}

				for it := k.IterateEmptyAt(b); it.NextBase(); {
					actualEmpty = append(actualEmpty, it.Value())
				}

				assert.Equal(t, expectedFilled, actualFilled, "filled bin mismatch")
				assert.Equal(t, expectedEmpty, actualEmpty, "empty bin mismatch")
			})
		}
	}
}

func TestIterate(t *testing.T) {
	cases := []struct {
		filled []Bin
		empty  []Bin
		label  string
	}{
		{
			filled: []Bin{2, 5, 12, 22, 24},
			empty:  []Bin{0, 9, 14, 17, 20, 26, 29},
			label:  "basic",
		},
		{
			filled: []Bin{},
			empty:  []Bin{15},
			label:  "none filled",
		},
		{
			filled: []Bin{15},
			empty:  []Bin{},
			label:  "all filled",
		},
		{
			filled: []Bin{7},
			empty:  []Bin{23},
			label:  "first half filled",
		},
		{
			filled: []Bin{23},
			empty:  []Bin{7},
			label:  "second half filled",
		},
	}

	for _, c := range cases {
		c := c
		t.Run(c.label, func(t *testing.T) {
			k := New()
			for _, i := range c.filled {
				k.Set(i)
			}

			actualFilled := []Bin{}
			actualEmpty := []Bin{}

			for it := k.IterateFilledAt(15); it.Next(); {
				actualFilled = append(actualFilled, it.Value())
			}

			for it := k.IterateEmptyAt(15); it.Next(); {
				actualEmpty = append(actualEmpty, it.Value())
			}

			assert.Equal(t, c.filled, actualFilled, "filled bin mismatch")
			assert.Equal(t, c.empty, actualEmpty, "empty bin mismatch")
		})
	}
}

func TestIterateAfter(t *testing.T) {
	filled := []Bin{2, 5, 12, 22, 24}
	start := Bin(12)

	expectedFilled := []Bin{12, 22, 24}
	expectedEmpty := []Bin{14, 17, 20, 26, 29}

	k := New()
	for _, i := range filled {
		k.Set(i)
	}

	actualFilled := []Bin{}
	actualEmpty := []Bin{}

	it := k.IterateFilledAt(15)
	for ok := it.NextAfter(start); ok; ok = it.Next() {
		actualFilled = append(actualFilled, it.Value())
	}

	it = k.IterateEmptyAt(15)
	for ok := it.NextAfter(start); ok; ok = it.Next() {
		actualEmpty = append(actualEmpty, it.Value())
	}

	assert.Equal(t, expectedFilled, actualFilled, "filled bin mismatch")
	assert.Equal(t, expectedEmpty, actualEmpty, "empty bin mismatch")
}

func TestIterateIntersectionBase(t *testing.T) {
	cases := []struct {
		filled0 []Bin
		filled1 []Bin
		bin     Bin
		label   string
	}{
		{
			filled0: []Bin{5, 9, 19, 24},
			filled1: []Bin{7, 17, 29},
			label:   "basic",
		},
		{
			filled0: []Bin{},
			filled1: []Bin{},
			label:   "none filled",
		},
		{
			filled0: []Bin{15},
			filled1: []Bin{15},
			label:   "all filled",
		},
		{
			filled0: []Bin{7},
			filled1: []Bin{23},
			label:   "no intersection",
		},
		{
			filled0: []Bin{7, 21, 25},
			filled1: []Bin{5, 9, 23},
			label:   "middles intersect",
		},
		{
			filled0: []Bin{0, 30},
			filled1: []Bin{0, 30},
			label:   "edges intersect",
		},
		{
			filled0: []Bin{7},
			filled1: []Bin{7},
			label:   "first half filled",
		},
		{
			filled0: []Bin{23},
			filled1: []Bin{23},
			label:   "second half filled",
		},
	}

	bins := []Bin{7, 15, 23}

	for _, c := range cases {
		c := c
		for _, b := range bins {
			b := b
			t.Run(fmt.Sprintf("%s, bin %d", c.label, b), func(t *testing.T) {
				k0 := New()
				for _, i := range c.filled0 {
					k0.Set(i)
				}

				k1 := New()
				for _, i := range c.filled1 {
					k1.Set(i)
				}

				expectedFilled := []Bin{}
				expectedEmpty := []Bin{}

				for i := b.BaseLeft(); i <= b.BaseRight(); i += 2 {
					if k0.FilledAt(i) && k1.FilledAt(i) {
						expectedFilled = append(expectedFilled, i)
					} else if k0.EmptyAt(i) && k1.EmptyAt(i) {
						expectedEmpty = append(expectedEmpty, i)
					}
				}

				actualFilled := []Bin{}
				actualEmpty := []Bin{}

				for it := NewIntersectionIterator(k0.IterateFilledAt(b), k1.IterateFilledAt(b)); it.NextBase(); {
					actualFilled = append(actualFilled, it.Value())
				}

				for it := NewIntersectionIterator(k0.IterateEmptyAt(b), k1.IterateEmptyAt(b)); it.NextBase(); {
					actualEmpty = append(actualEmpty, it.Value())
				}

				assert.Equal(t, expectedFilled, actualFilled, "filled bin mismatch")
				assert.Equal(t, expectedEmpty, actualEmpty, "empty bin mismatch")
			})
		}
	}
}

func TestIterateIntersection(t *testing.T) {
	cases := []struct {
		filled0     []Bin
		filled1     []Bin
		root        Bin
		filled      []Bin
		difference0 []Bin
		difference1 []Bin
		bin         Bin
		label       string
	}{
		{
			filled0:     []Bin{5, 9, 19, 24},
			filled1:     []Bin{7, 17, 29},
			root:        15,
			filled:      []Bin{5, 9, 17},
			difference0: []Bin{1, 13, 29},
			difference1: []Bin{21, 24},
			label:       "basic",
		},
		{
			filled0:     []Bin{},
			filled1:     []Bin{},
			root:        15,
			filled:      []Bin{},
			difference0: []Bin{},
			difference1: []Bin{},
			label:       "none filled",
		},
		{
			filled0:     []Bin{15},
			filled1:     []Bin{15},
			root:        15,
			filled:      []Bin{15},
			difference0: []Bin{},
			difference1: []Bin{},
			label:       "all filled",
		},
		{
			filled0:     []Bin{7},
			filled1:     []Bin{23},
			root:        15,
			filled:      []Bin{},
			difference0: []Bin{23},
			difference1: []Bin{7},
			label:       "no intersection",
		},
		{
			filled0:     []Bin{7, 21, 25},
			filled1:     []Bin{5, 9, 23},
			root:        15,
			filled:      []Bin{5, 9, 21, 25},
			difference0: []Bin{17, 29},
			difference1: []Bin{1, 13},
			label:       "middles intersect",
		},
		{
			filled0:     []Bin{0, 30},
			filled1:     []Bin{0, 30},
			root:        15,
			filled:      []Bin{0, 30},
			difference0: []Bin{},
			difference1: []Bin{},
			label:       "edges intersect",
		},
		{
			filled0:     []Bin{7},
			filled1:     []Bin{7},
			root:        15,
			filled:      []Bin{7},
			difference0: []Bin{},
			difference1: []Bin{},
			label:       "first half filled",
		},
		{
			filled0:     []Bin{23},
			filled1:     []Bin{23},
			root:        15,
			filled:      []Bin{23},
			difference0: []Bin{},
			difference1: []Bin{},
			label:       "second half filled",
		},
		{
			filled0:     []Bin{255, 614},
			filled1:     []Bin{23},
			root:        511,
			filled:      []Bin{23},
			difference0: []Bin{},
			difference1: []Bin{7, 47, 95, 191, 383, 614},
			label:       "wide",
		},
	}

	for _, c := range cases {
		c := c
		t.Run(c.label, func(t *testing.T) {
			k0 := New()
			for _, i := range c.filled0 {
				k0.Set(i)
			}

			k1 := New()
			for _, i := range c.filled1 {
				k1.Set(i)
			}

			actualFilled := []Bin{}
			actualDifference0 := []Bin{}
			actualDifference1 := []Bin{}

			for it := NewIntersectionIterator(k0.IterateFilledAt(c.root), k1.IterateFilledAt(c.root)); it.Next(); {
				actualFilled = append(actualFilled, it.Value())
			}
			for it := NewIntersectionIterator(k0.IterateEmptyAt(c.root), k1.IterateFilledAt(c.root)); it.Next(); {
				actualDifference0 = append(actualDifference0, it.Value())
			}
			for it := NewIntersectionIterator(k0.IterateFilledAt(c.root), k1.IterateEmptyAt(c.root)); it.Next(); {
				actualDifference1 = append(actualDifference1, it.Value())
			}

			assert.Equal(t, c.filled, actualFilled, "filled bin mismatch")
			assert.Equal(t, c.difference0, actualDifference0, "difference 1-0 bin mismatch")
			assert.Equal(t, c.difference1, actualDifference1, "difference 0-1 bin mismatch")
		})
	}
}

func TestIterateIntersectionAfter(t *testing.T) {
	filled0 := []Bin{11, 28}
	filled1 := []Bin{15}
	root := Bin(15)
	first := Bin(12)
	expected := []Bin{13, 28}

	k0 := New()
	for _, i := range filled0 {
		k0.Set(i)
	}

	k1 := New()
	for _, i := range filled1 {
		k1.Set(i)
	}

	actual := []Bin{}

	it := NewIntersectionIterator(k0.IterateFilledAt(root), k1.IterateFilledAt(root))
	for ok := it.NextAfter(first); ok; ok = it.Next() {
		actual = append(actual, it.Value())
	}

	assert.Equal(t, expected, actual, "filled bin mismatch")
}

func TestIterateBaseAfter(t *testing.T) {
	filled := []Bin{11, 28}

	k := New()
	for _, i := range filled {
		k.Set(i)
	}

	cases := []struct {
		first    Bin
		expected []Bin
	}{
		{
			first:    0,
			expected: []Bin{8, 10, 12, 14, 28},
		},
		{
			first:    12,
			expected: []Bin{12, 14, 28},
		},
		{
			first:    30,
			expected: []Bin{},
		},
	}

	for _, c := range cases {
		actual := []Bin{}

		it := k.IterateFilled()
		for ok := it.NextBaseAfter(c.first); ok; ok = it.NextBase() {
			actual = append(actual, it.Value())
		}

		assert.Equal(t, c.expected, actual, "filled bin mismatch")
	}
}

func TestIteratorToSlice(t *testing.T) {
	filled := []Bin{5, 11, 28, 129}

	k := New()
	for _, i := range filled {
		k.Set(i)
	}

	assert.Equal(t, filled, k.IterateFilled().ToSlice())
}
