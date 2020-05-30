package event

import (
	"reflect"
	"sync"
)

// Observable ...
type Observable struct {
	observers sync.Map
}

// Notify ...
func (o *Observable) Notify(ch interface{}) {
	o.observers.Store(ch, ch)
}

// StopNotifying ...
func (o *Observable) StopNotifying(ch interface{}) {
	o.observers.Delete(ch)
}

// Emit ...
func (o *Observable) Emit(v interface{}) {
	o.observers.Range(func(_ interface{}, chi interface{}) bool {
		reflect.Select([]reflect.SelectCase{
			{
				Dir:  reflect.SelectSend,
				Chan: reflect.ValueOf(chi),
				Send: reflect.ValueOf(v),
			},
			{Dir: reflect.SelectDefault},
		})
		return true
	})
}
