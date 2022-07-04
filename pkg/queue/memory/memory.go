// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package memory

import (
	"sync"

	"github.com/MemeLabs/strims/pkg/queue"
	"github.com/MemeLabs/strims/pkg/syncutil"
)

func NewTransport() *Transport {
	return &Transport{}
}

type Transport struct {
	qs syncutil.Set[*Queue[any]]
}

func (t *Transport) Open(name string) (queue.Queue, error) {
	q := NewQueue[any]()
	q.closeHook = func() { t.qs.Delete(q) }
	t.qs.Insert(q)
	return q, nil
}

func (t *Transport) Close() error {
	for _, q := range t.qs.Values() {
		q.Close()
	}
	return nil
}

func NewQueue[T any]() *Queue[T] {
	return &Queue[T]{
		ch:    make(chan struct{}, 1),
		close: make(chan struct{}),
		queue: queue.NewRing[T](16),
	}
}

type Queue[T any] struct {
	ch        chan struct{}
	closeOnce sync.Once
	closeHook func()
	close     chan struct{}
	lock      sync.Mutex
	queue     queue.Ring[T]
}

func (t *Queue[T]) Write(e T) error {
	t.lock.Lock()
	t.queue.Push(e)
	l := t.queue.Len()
	t.lock.Unlock()

	if l > 1 {
		return nil
	}

	select {
	case t.ch <- struct{}{}:
	case <-t.close:
		return queue.ErrTransportClosed
	default:
	}
	return nil
}

func (t *Queue[T]) Read() (T, error) {
	var e T

	t.lock.Lock()
	e, ok := t.queue.PopFront()
	t.lock.Unlock()

	if ok {
		return e, nil
	}

	select {
	case <-t.ch:
		return t.Read()
	case <-t.close:
		return e, queue.ErrTransportClosed
	}
}

func (t *Queue[T]) Close() error {
	t.closeOnce.Do(func() {
		if t.closeHook != nil {
			t.closeHook()
		}
		close(t.close)
	})
	return nil
}
