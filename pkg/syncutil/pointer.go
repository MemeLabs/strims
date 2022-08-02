// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package syncutil

import (
	"sync/atomic"
)

func NewPointer[T any](val *T) (p atomic.Pointer[T]) {
	p.Store(val)
	return
}
