// Code generated by SQLBoiler 4.6.0 (https://github.com/volatiletech/sqlboiler). DO NOT EDIT.
// This file is meant to be re-generated in place and/or deleted at any time.

package models

import "testing"

// This test suite runs each operation test in parallel.
// Example, if your database has 3 tables, the suite will run:
// table1, table2 and table3 Delete in parallel
// table1, table2 and table3 Insert in parallel, and so forth.
// It does NOT run each operation group in parallel.
// Separating the tests thusly grants avoidance of Postgres deadlocks.
func TestParent(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeers)
	t.Run("Nodes", testNodes)
}

func TestDelete(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersDelete)
	t.Run("Nodes", testNodesDelete)
}

func TestQueryDeleteAll(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersQueryDeleteAll)
	t.Run("Nodes", testNodesQueryDeleteAll)
}

func TestSliceDeleteAll(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersSliceDeleteAll)
	t.Run("Nodes", testNodesSliceDeleteAll)
}

func TestExists(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersExists)
	t.Run("Nodes", testNodesExists)
}

func TestFind(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersFind)
	t.Run("Nodes", testNodesFind)
}

func TestBind(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersBind)
	t.Run("Nodes", testNodesBind)
}

func TestOne(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersOne)
	t.Run("Nodes", testNodesOne)
}

func TestAll(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersAll)
	t.Run("Nodes", testNodesAll)
}

func TestCount(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersCount)
	t.Run("Nodes", testNodesCount)
}

func TestInsert(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersInsert)
	t.Run("ExternalPeers", testExternalPeersInsertWhitelist)
	t.Run("Nodes", testNodesInsert)
	t.Run("Nodes", testNodesInsertWhitelist)
}

// TestToOne tests cannot be run in parallel
// or deadlocks can occur.
func TestToOne(t *testing.T) {}

// TestOneToOne tests cannot be run in parallel
// or deadlocks can occur.
func TestOneToOne(t *testing.T) {}

// TestToMany tests cannot be run in parallel
// or deadlocks can occur.
func TestToMany(t *testing.T) {}

// TestToOneSet tests cannot be run in parallel
// or deadlocks can occur.
func TestToOneSet(t *testing.T) {}

// TestToOneRemove tests cannot be run in parallel
// or deadlocks can occur.
func TestToOneRemove(t *testing.T) {}

// TestOneToOneSet tests cannot be run in parallel
// or deadlocks can occur.
func TestOneToOneSet(t *testing.T) {}

// TestOneToOneRemove tests cannot be run in parallel
// or deadlocks can occur.
func TestOneToOneRemove(t *testing.T) {}

// TestToManyAdd tests cannot be run in parallel
// or deadlocks can occur.
func TestToManyAdd(t *testing.T) {}

// TestToManySet tests cannot be run in parallel
// or deadlocks can occur.
func TestToManySet(t *testing.T) {}

// TestToManyRemove tests cannot be run in parallel
// or deadlocks can occur.
func TestToManyRemove(t *testing.T) {}

func TestReload(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersReload)
	t.Run("Nodes", testNodesReload)
}

func TestReloadAll(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersReloadAll)
	t.Run("Nodes", testNodesReloadAll)
}

func TestSelect(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersSelect)
	t.Run("Nodes", testNodesSelect)
}

func TestUpdate(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersUpdate)
	t.Run("Nodes", testNodesUpdate)
}

func TestSliceUpdateAll(t *testing.T) {
	t.Run("ExternalPeers", testExternalPeersSliceUpdateAll)
	t.Run("Nodes", testNodesSliceUpdateAll)
}
