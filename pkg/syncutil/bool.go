package syncutil

import "sync/atomic"

type Bool uint32

func (b *Bool) Get() bool {
	return atomic.LoadUint32((*uint32)(b)) == 1
}

func (b *Bool) Set(v bool) {
	if v {
		atomic.StoreUint32((*uint32)(b), 1)
	} else {
		atomic.StoreUint32((*uint32)(b), 0)
	}
}
