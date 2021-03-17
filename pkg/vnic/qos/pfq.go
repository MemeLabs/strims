package qos

// H-PFQ (Hierarchical Packet Fair Queue) from
//  Jon C. R. Bennett, Member, IEEE, and Hui Zhang, "Hierarchical
//  Packet Fair Queueing Algorithms"
//
//  https://www.academia.edu/download/48781686/90.64956820160912-12644-1bd1p1k.pdf

// MaxWeight ...
const MaxWeight = 1000

type pfqBase struct {
	head   Packet
	parent *pfqNode
	weight uint64
	stime  uint64
	ftime  uint64
}

func (q *pfqBase) updateStartFinish(useFTime bool) {
	if !useFTime && q.parent != nil {
		q.stime = maxUint64(q.ftime, q.parent.vtime)
	} else {
		q.stime = q.ftime
	}

	q.ftime = q.stime + q.head.Size()*q.weight
}

func (q *pfqBase) startTime() uint64 {
	return q.stime
}

func (q *pfqBase) finishTime() uint64 {
	return q.ftime
}

func (q *pfqBase) Head() Packet {
	return q.head
}

type pfqNode struct {
	pfqBase
	queue       Queue
	activeChild PFQ
	vtime       uint64
	busy        bool
}

func (q *pfqNode) Dequeue() Packet {
	defer q.resetPath()
	return q.head
}

func (q *pfqNode) restartNode() {
	q.activeChild = q.queue.SelectNext(q.vtime)
	if q.activeChild != nil {
		q.head = q.activeChild.Head()

		q.updateStartFinish(q.busy)
		if q.parent != nil {
			q.parent.queue.Enqueue(q)
		}
		q.busy = true

		q.vtime = q.queue.ComputeVirtualTime(q.vtime, q.head.Size()*MaxWeight)
	} else {
		q.busy = false
	}

	if q.parent != nil && q.parent.head == nil {
		q.parent.restartNode()
	}
}

func (q *pfqNode) resetPath() {
	q.head = nil
	m := q.activeChild
	q.activeChild = nil
	m.resetPath()
}

type pfqLeaf struct {
	pfqBase
	queue PacketQueue
}

func (q *pfqLeaf) Arrive(p Packet) {
	if q.head != nil {
		q.queue.Enqueue(p)
		return
	}

	q.head = p
	q.updateStartFinish(false)
	q.parent.queue.Enqueue(q)

	if !q.parent.busy {
		q.parent.restartNode()
	}
}

func (q *pfqLeaf) Close() {
	if q.head == nil {
		return
	}

	q.parent.queue.Delete(q)
	q.head = &noopPacket{}
	q.queue.Clear()
}

func (q *pfqLeaf) resetPath() {
	q.head = q.queue.Dequeue()
	if q.head != nil {
		q.updateStartFinish(true)
		q.parent.queue.Enqueue(q)
	}
	q.parent.restartNode()
}

type noopPacket struct{}

func (p *noopPacket) Size() uint64 {
	return 0
}

func (p *noopPacket) Send() {}
