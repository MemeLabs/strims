package sortutil

import (
	"sort"

	"golang.org/x/exp/constraints"
)

// Ordered ...
func Ordered[T constraints.Ordered](a []T) {
	sort.Sort(OrderedSlice[T](a))
}

// OrderedSlice ...
type OrderedSlice[T constraints.Ordered] []T

func (a OrderedSlice[T]) Len() int           { return len(a) }
func (a OrderedSlice[T]) Swap(i, j int)      { a[i], a[j] = a[j], a[i] }
func (a OrderedSlice[T]) Less(i, j int) bool { return a[i] < a[j] }
