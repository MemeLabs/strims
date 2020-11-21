package event

import "github.com/MemeLabs/go-ppspp/pkg/event"

// Observers ...
type Observers struct {
	CA    event.Observer
	Network   event.Observer
	Peer  event.Observer
	Swarm event.Observer
}
