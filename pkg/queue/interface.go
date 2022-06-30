// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package queue

import "errors"

var ErrTransportClosed = errors.New("transport closed")

type Transport interface {
	Open(name string) (Queue, error)
	Close() error
}

type Queue interface {
	Write(e any) error
	Read() (any, error)
	Close() error
}
