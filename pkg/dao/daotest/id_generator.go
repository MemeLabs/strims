package daotest

import "sync/atomic"

var nextID uint64 = 1

// IDGenreator ...
type IDGenreator struct{}

// GenerateID ...
func (g *IDGenreator) GenerateID() (uint64, error) {
	return atomic.AddUint64(&nextID, 1), nil
}
