// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package servicemanager

import (
	"context"
)

func NewNoOpClient[T any]() (*NoOpClient[T], error) {
	return &NoOpClient[T]{}, nil
}

type NoOpClient[T any] struct {
	stopper Stopper
}

func (d *NoOpClient[T]) Run(ctx context.Context) error {
	done, ctx := d.stopper.Start(ctx)
	defer done()

	<-ctx.Done()

	return ctx.Err()
}

func (d *NoOpClient[T]) Close(ctx context.Context) error {
	select {
	case <-d.stopper.Stop():
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (d *NoOpClient[T]) Reader(ctx context.Context) (r T, err error) {
	return
}
