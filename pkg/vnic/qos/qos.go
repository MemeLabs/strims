// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package qos

// PFQ ...
type PFQ interface {
	resetPath()
	startTime() uint64
	finishTime() uint64
	Head() Packet
}

// Queue ...
type Queue interface {
	Enqueue(p PFQ)
	Delete(p PFQ)
	SelectNext(vtime uint64) PFQ
	ComputeVirtualTime(vtime, work uint64) uint64
}

// PacketQueue ...
type PacketQueue interface {
	Enqueue(p Packet)
	Dequeue() Packet
	Clear()
}

// Packet ...
type Packet interface {
	Size() uint64
	Send()
}

func maxUint64(a, b uint64) uint64 {
	if a > b {
		return a
	}
	return b
}
