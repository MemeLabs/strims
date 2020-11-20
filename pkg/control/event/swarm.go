package event

import "github.com/MemeLabs/go-ppspp/pkg/ppspp"

// SwarmAdd ...
type SwarmAdd struct {
	Swarm *ppspp.Swarm
}

// SwarmRemove ...
type SwarmRemove struct {
	Swarm *ppspp.Swarm
}
