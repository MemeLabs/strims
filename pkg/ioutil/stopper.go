// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ioutil

import "errors"

// based on stoppable reader proposal by Liam Breck
// see: https://github.com/golang/go/issues/36402

var ErrStopped = errors.New("read stopped")

type Stopper <-chan struct{}

type ReadStopper interface {
	SetReadStopper(Stopper)
}

type WriteStopper interface {
	SetWriteStopper(Stopper)
}
