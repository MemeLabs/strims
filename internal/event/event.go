package event

import (
	"github.com/MemeLabs/go-ppspp/pkg/event"
)

// Observers ...
type Observers struct {
	global event.Observer
	local  event.Observer
}

// Notify ...
func (o *Observers) Notify(ch interface{}) {
	o.global.Notify(ch)
	o.local.Notify(ch)
}

// StopNotifying ...
func (o *Observers) StopNotifying(ch interface{}) {
	o.global.StopNotifying(ch)
	o.local.StopNotifying(ch)
}

// EmitGlobal ...
func (o *Observers) EmitGlobal(v interface{}) {
	o.global.Emit(v)
}

// EmitLocal ...
func (o *Observers) EmitLocal(v interface{}) {
	o.local.Emit(v)
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

// func (o *Observers) AddHandlerWithPriority(priority int, h interface{}) {
// 	o.global.AddHandlerWithPriority(priority, h)
// 	o.local.AddHandlerWithPriority(priority, h)
// }

// func (o *Observers) AddHandler(h interface{}) {
// 	o.global.AddHandler(h)
// 	o.local.AddHandler(h)
// }

// func (o *Observers) RemvoeHandler(h interface{}) {
// 	o.global.RemoveHandler(h)
// 	o.local.RemoveHandler(h)
// }

// func (o *Observers) EmitGlobal(v interface{}) {
// 	o.global.Emit(v)
// }

// func (o *Observers) EmitLocal(v interface{}) {
// 	o.local.Emit(v)
// }
