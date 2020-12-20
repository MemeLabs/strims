package daotest

import "sync/atomic"

var nextID uint64 = 1

// IDGenerator ...
type IDGenerator struct{}

// GenerateID ...
func (g *IDGenerator) GenerateID() (uint64, error) {
	return atomic.AddUint64(&nextID, 1), nil
}
