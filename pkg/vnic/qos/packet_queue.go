// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package qos

import "sync"

var listPacketQueueItemPool = sync.Pool{
	New: func() any {
		return &listPacketQueueItem{}
	},
}

type listPacketQueue struct {
	head, tail *listPacketQueueItem
}

func (q *listPacketQueue) Enqueue(p Packet) {
	i := listPacketQueueItemPool.Get().(*listPacketQueueItem)
	i.next = nil
	i.p = p

	if q.head == nil {
		q.head = i
	}

	if q.tail != nil {
		q.tail.next = i
	}
	q.tail = i
}

func (q *listPacketQueue) Dequeue() Packet {
	if q.head == nil {
		return nil
	}

	h := q.head

	q.head = h.next

	if q.tail == h {
		q.tail = nil
	}

	defer listPacketQueueItemPool.Put(h)
	return h.p
}

func (q *listPacketQueue) Clear() {
	for q.Dequeue() != nil {
	}
}

type listPacketQueueItem struct {
	next *listPacketQueueItem
	p    Packet
}
