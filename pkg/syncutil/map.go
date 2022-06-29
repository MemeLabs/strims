// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package syncutil

import "sync"

type Map[K comparable, V any] struct {
	mu sync.Mutex
	m  map[K]V
}

func (m *Map[K, V]) Get(k K) (v V, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	v, ok = m.m[k]
	return
}

func (m *Map[K, V]) Set(k K, v V) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.m == nil {
		m.m = map[K]V{}
	}
	m.m[k] = v
}

func (m *Map[K, V]) Delete(k K) {
	m.mu.Lock()
	defer m.mu.Unlock()
	delete(m.m, k)
}

func (m *Map[K, V]) Clear() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.m = nil
}

func (m *Map[K, V]) Each(it func(k K, v V)) {
	m.mu.Lock()
	defer m.mu.Unlock()
	for k, v := range m.m {
		it(k, v)
	}
}

func (m *Map[K, V]) Keys() (ks []K) {
	m.Each(func(k K, v V) {
		ks = append(ks, k)
	})
	return
}

func (m *Map[K, V]) Values() (vs []V) {
	m.Each(func(k K, v V) {
		vs = append(vs, v)
	})
	return
}
