// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"log"
	"sync"
	"time"
)

// Print print message to log if stalled
func Print(v ...any) func() {
	return func() { log.Println(v...) }
}

// Printf print formatted message to log if stalled
func Printf(format string, v ...any) func() {
	return func() { log.Printf(format, v...) }
}

// Panic print message and panic if stalled
func Panic(v ...any) func() {
	return func() { log.Panicln(v...) }
}

// IfStalled execute some handlers unless cancel is called
// ex.
// defer IfStalled(Print("function did not return..."))()
func IfStalled(handlers ...func()) (cancel func()) {
	t := time.After(time.Second)
	c := make(chan struct{}, 1)

	go func() {
		select {
		case <-t:
			for _, fn := range handlers {
				fn()
			}
		case <-c:
		}
	}()

	var cancelOnce sync.Once
	return func() {
		cancelOnce.Do(func() {
			select {
			case c <- struct{}{}:
			default:
			}
			close(c)
		})
	}
}
