package event

import (
	"fmt"
	"log"
	"reflect"
	"runtime"
	"sync"
	"time"
)

// Observer ...
type Observer struct {
	observers sync.Map
	callers   sync.Map
}

// Notify ...
func (o *Observer) Notify(ch any) {
	o.observers.Store(ch, ch)
	_, file, line, _ := runtime.Caller(2)
	o.callers.Store(ch, fmt.Sprintf("%s:%d", file, line))
}

// StopNotifying ...
func (o *Observer) StopNotifying(ch any) {
	o.observers.Delete(ch)
}

// Emit ...
func (o *Observer) Emit(v any) {
	t := time.NewTimer(time.Second)
	done := make(chan struct{})
	defer t.Stop()
	o.observers.Range(func(_ any, chi any) bool {
		go func() {
			select {
			case <-t.C:
				caller, _ := o.callers.Load(chi)
				log.Panicf("froze in channel registered at %s", caller.(string))
			case <-done:
			}
		}()
		reflect.ValueOf(chi).Send(reflect.ValueOf(v))
		done <- struct{}{}
		return true
	})
}

// Close ...
func (o *Observer) Close() {
	o.observers.Range(func(_ any, chi any) bool {
		reflect.ValueOf(chi).Close()
		return true
	})
}
