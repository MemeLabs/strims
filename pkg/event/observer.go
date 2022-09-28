// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	"fmt"
	"log"
	"reflect"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
)

// Observer ...
type Observer struct {
	observers sync.Map
	callers   sync.Map
	size      int32
}

func (o *Observer) Size() int {
	return int(atomic.LoadInt32(&o.size))
}

// Notify ...
func (o *Observer) Notify(ch any) {
	o.observers.Store(ch, reflect.ValueOf(ch))
	_, file, line, _ := runtime.Caller(2)
	o.callers.Store(ch, fmt.Sprintf("%s:%d", file, line))
	atomic.AddInt32(&o.size, 1)
}

// StopNotifying ...
func (o *Observer) StopNotifying(ch any) {
	o.observers.Delete(ch)
	atomic.AddInt32(&o.size, -1)
}

// Emit ...
func (o *Observer) Emit(v any) {
	t := time.NewTimer(time.Second)
	done := make(chan struct{})
	defer t.Stop()
	o.observers.Range(func(chi any, chv any) bool {
		go func() {
			select {
			case <-t.C:
				caller, _ := o.callers.Load(chi)
				log.Panicf("froze in channel registered at %s", caller.(string))
			case <-done:
			}
		}()
		chv.(reflect.Value).Send(reflect.ValueOf(v))
		done <- struct{}{}
		return true
	})
}

// Close ...
func (o *Observer) Close() {
	o.observers.Range(func(chi any, chv any) bool {
		o.StopNotifying(chi)
		chv.(reflect.Value).Close()
		return true
	})
}
