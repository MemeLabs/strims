package lock

import "sync"

// ex .
// func ex(a, b sync.Locker) {
// 	defer lock(a, b)()
// 	...
// }
func do(ls ...sync.Locker) (unlock func()) {
	for _, l := range ls {
		l.Lock()
	}
	return func() {
		for i := len(ls) - 1; i >= 0; i-- {
			ls[i].Unlock()
		}
	}
}
