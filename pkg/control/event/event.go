package event

import "github.com/MemeLabs/go-ppspp/pkg/event"

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

// EmitGlobal ...
func (o *Observers) EmitGlobal(v interface{}) {
	o.global.Emit(v)
}

// EmitLocal ...
func (o *Observers) EmitLocal(v interface{}) {
	o.local.Emit(v)
}
