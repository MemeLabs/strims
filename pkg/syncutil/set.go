// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package syncutil

type Set[V comparable] struct {
	m Map[V, struct{}]
}

func (s *Set[V]) Has(v V) bool {
	_, ok := s.m.Get(v)
	return ok
}

func (s *Set[V]) Insert(v V) {
	s.m.Set(v, struct{}{})
}

func (s *Set[V]) Delete(v V) {
	s.m.Delete(v)
}

func (s *Set[V]) Clear() {
	s.m.Clear()
}

func (s *Set[V]) Each(it func(v V)) {
	s.m.Each(func(v V, _ struct{}) {
		it(v)
	})
}

func (s *Set[V]) Values() []V {
	return s.m.Keys()
}
