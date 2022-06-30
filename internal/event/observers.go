// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	"errors"

	"github.com/MemeLabs/strims/pkg/event"
	"github.com/MemeLabs/strims/pkg/queue"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/anypb"
)

type Cipher interface {
	Seal(p []byte) ([]byte, error)
	Open(b []byte) ([]byte, error)
}

func NewObservers(logger *zap.Logger, q queue.Queue, c Cipher) *Observers {
	o := &Observers{
		logger: logger,
		c:      c,
		q:      q,
	}

	go o.readQueue()

	return o
}

// Observers ...
type Observers struct {
	logger *zap.Logger
	c      Cipher
	q      queue.Queue
	global event.Observer
	local  event.Observer
}

func (o *Observers) readQueue() {
	for {
		e, err := o.readQueueEvent()
		if errors.Is(err, queue.ErrTransportClosed) {
			return
		} else if err != nil {
			o.logger.Error("reading global event queue", zap.Error(err))
			continue
		}
		o.global.Emit(e)
	}
}

func (o *Observers) readQueueEvent() (proto.Message, error) {
	e, err := o.q.Read()
	if err != nil {
		return nil, err
	}
	b, ok := e.([]byte)
	if !ok {
		return nil, err
	}
	b, err = o.c.Open(b)
	if err != nil {
		return nil, err
	}
	var a anypb.Any
	if err = proto.Unmarshal(b, &a); err != nil {
		return nil, err
	}
	m, err := anypb.UnmarshalNew(&a, proto.UnmarshalOptions{})
	if err != nil {
		return nil, err
	}
	return m, nil
}

func (o *Observers) writeQueueEvent(m proto.Message) error {
	a, err := anypb.New(m)
	if err != nil {
		return err
	}
	b, err := proto.Marshal(a)
	if err != nil {
		return err
	}
	b, err = o.c.Seal(b)
	if err != nil {
		return err
	}
	return o.q.Write(b)
}

// Emit implements dao.EventEmitter
func (o *Observers) Emit(v proto.Message) {
	o.EmitGlobal(v)
}

// EmitGlobal ...
func (o *Observers) EmitGlobal(v proto.Message) {
	if err := o.writeQueueEvent(v); err != nil {
		o.logger.Error("emitting global event", zap.Error(err))
	}
}

// EmitLocal ...
func (o *Observers) EmitLocal(v any) {
	o.local.Emit(v)
}

func (o *Observers) Chan() chan any {
	ch := make(chan any, 8)
	o.global.Notify(ch)
	o.local.Notify(ch)
	return ch
}

func (o *Observers) Events() (chan any, func()) {
	ch := make(chan any, 8)
	o.global.Notify(ch)
	o.local.Notify(ch)
	return ch, func() { o.StopNotifying(ch) }
}

// Notify ...
func (o *Observers) Notify(ch any) {
	o.global.Notify(ch)
	o.local.Notify(ch)
}

// StopNotifying ...
func (o *Observers) StopNotifying(ch any) {
	o.global.StopNotifying(ch)
	o.local.StopNotifying(ch)
}
