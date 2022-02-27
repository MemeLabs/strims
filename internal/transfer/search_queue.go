package transfer

func newSearchQueue(n int) *searchQueue {
	return &searchQueue{
		items: make([][]searchQueueItem, n),
	}
}

type searchQueue struct {
	i     int
	items [][]searchQueueItem
}

type searchQueueItem struct {
	transfer *transfer
	network  *network
}

func (q *searchQueue) Next() []searchQueueItem {
	q.i = (q.i + 1) % len(q.items)
	return q.items[q.i]
}

func (q *searchQueue) Insert(t *transfer, n *network) {
	q.items[q.i] = append(q.items[q.i], searchQueueItem{t, n})
}

func (q *searchQueue) delete(f func(it searchQueueItem) bool) {
	for i := range q.items {
		for j := 0; j < len(q.items[i]); j++ {
			if f(q.items[i][j]) {
				l := len(q.items[i]) - 1
				q.items[i][j] = q.items[i][l]
				q.items[i][l] = searchQueueItem{}
				q.items[i] = q.items[i][:l]
				j--
			}
		}
	}
}

func (q *searchQueue) DeleteTransfer(t *transfer) {
	q.delete(func(it searchQueueItem) bool { return it.transfer == t })
}

func (q *searchQueue) DeleteNetwork(n *network) {
	q.delete(func(it searchQueueItem) bool { return it.network == n })
}
