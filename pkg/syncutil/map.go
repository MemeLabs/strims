// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package syncutil

import "sync"

type Map[K comparable, V any] struct {
	mu sync.Mutex
	m  map[K]V
}

func (m *Map[K, V]) Len() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return len(m.m)
}

func (m *Map[K, V]) Get(k K) (v V, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	v, ok = m.m[k]
	return
}

func (m *Map[K, V]) Has(k K) (ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	_, ok = m.m[k]
	return
}

func (m *Map[K, V]) GetAndDelete(k K) (v V, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	v, ok = m.m[k]
	delete(m.m, k)
	return
}

func (m *Map[K, V]) GetOrInsert(k K, iv V) (v V, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.m == nil {
		m.m = map[K]V{}
	}
	if v, ok = m.m[k]; ok {
		return v, true
	}
	m.m[k] = iv
	return iv, false
}

func (m *Map[K, V]) InsertOrReplace(k K, iv V) (v V, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.m == nil {
		m.m = map[K]V{}
	}
	if v, ok = m.m[k]; !ok {
		v = iv
	}
	m.m[k] = iv
	return v, ok
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
