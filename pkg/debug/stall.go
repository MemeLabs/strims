package debug

import (
	"log"
	"time"
)

// Print print message to log if stalled
func Print(v ...interface{}) func() {
	return func() { log.Println(v...) }
}

// Panic print message and panic if stalled
func Panic(v ...interface{}) func() {
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

	return func() {
		c <- struct{}{}
		close(c)
	}
}
