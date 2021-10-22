package event

import (
	"reflect"
	"sync"
)

// Observer ...
type Observer struct {
	observers sync.Map
}

// Notify ...
func (o *Observer) Notify(ch interface{}) {
	o.observers.Store(ch, ch)
}

// StopNotifying ...
func (o *Observer) StopNotifying(ch interface{}) {
	o.observers.Delete(ch)
}

// Emit ...
func (o *Observer) Emit(v interface{}) {
	o.observers.Range(func(_ interface{}, chi interface{}) bool {
		reflect.ValueOf(chi).Send(reflect.ValueOf(v))
		return true
	})
}

// Close ...
func (o *Observer) Close() {
	o.observers.Range(func(_ interface{}, chi interface{}) bool {
		reflect.ValueOf(chi).Close()
		return true
	})
}
