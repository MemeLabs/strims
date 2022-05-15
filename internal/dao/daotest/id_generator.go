// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package daotest

import "sync/atomic"

var nextID uint64 = 1

// IDGenerator ...
type IDGenerator struct{}

// GenerateID ...
func (g *IDGenerator) GenerateID() (uint64, error) {
	return atomic.AddUint64(&nextID, 1), nil
}
