// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	"github.com/MemeLabs/strims/pkg/event"
	"google.golang.org/protobuf/proto"
)

// Observers ...
type Observers struct {
	global event.Observer
	local  event.Observer
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

// EmitGlobal ...
func (o *Observers) EmitGlobal(v any) {
	o.global.Emit(v)
}

// EmitLocal ...
func (o *Observers) EmitLocal(v any) {
	o.local.Emit(v)
}

// Emit implements dao.EventEmitter
func (o *Observers) Emit(v proto.Message) {
	o.global.Emit(v)
}

// func NewObservers() *Observers {
// 	return &Observers{
// 		global: event.NewEmitter(event.NewMemoryTransport()),
// 		local:  event.NewEmitter(event.NewMemoryTransport()),
// 	}
// }

// type Observers struct {
// 	global *event.Emitter
// 	local  *event.Emitter
// }

// func (o *Observers) AddHandlerWithPriority(priority int, h any) {
// 	o.global.AddHandlerWithPriority(priority, h)
// 	o.local.AddHandlerWithPriority(priority, h)
// }

// func (o *Observers) AddHandler(h any) {
// 	o.global.AddHandler(h)
// 	o.local.AddHandler(h)
// }

// func (o *Observers) RemvoeHandler(h any) {
// 	o.global.RemoveHandler(h)
// 	o.local.RemoveHandler(h)
// }

// func (o *Observers) EmitGlobal(v any) {
// 	o.global.Emit(v)
// }

// func (o *Observers) EmitLocal(v any) {
// 	o.local.Emit(v)
// }
