package syncutil

import "sync"

type Map[K comparable, V any] struct {
	mu sync.Mutex
	m  map[K]V
}

func (m *Map[K, V]) Get(k K) (v V, ok bool) {
	m.mu.Lock()
	v, ok = m.m[k]
	m.mu.Unlock()
	return
}

func (m *Map[K, V]) Set(k K, v V) {
	m.mu.Lock()
	if m.m == nil {
		m.m = map[K]V{}
	}
	m.m[k] = v
	m.mu.Unlock()
}

func (m *Map[K, V]) Delete(k K) {
	m.mu.Lock()
	delete(m.m, k)
	m.mu.Unlock()
}
