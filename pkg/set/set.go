package set

import (
	"constraints"
)

func New[T constraints.Ordered](size int) *Set[T] {
	return &Set[T]{
		values: make(map[T]struct{}, size),
	}
}

type Set[T constraints.Ordered] struct {
	values map[T]struct{}
}

func (s *Set[T]) Insert(v T) {
	if _, ok := s.values[v]; !ok {
		s.values[v] = struct{}{}
	}
}

func (s *Set[T]) Slice() []T {
	vs := make([]T, 0, len(s.values))
	for v := range s.values {
		vs = append(vs, v)
	}
	return vs
}
