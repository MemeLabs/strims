// Code generated by SQLBoiler 4.2.0 (https://github.com/volatiletech/sqlboiler). DO NOT EDIT.
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
	t.Run("Subplans", testSubplans)
	t.Run("Subscriptions", testSubscriptions)
	t.Run("Transactions", testTransactions)
}

func TestDelete(t *testing.T) {
	t.Run("Subplans", testSubplansDelete)
	t.Run("Subscriptions", testSubscriptionsDelete)
	t.Run("Transactions", testTransactionsDelete)
}

func TestQueryDeleteAll(t *testing.T) {
	t.Run("Subplans", testSubplansQueryDeleteAll)
	t.Run("Subscriptions", testSubscriptionsQueryDeleteAll)
	t.Run("Transactions", testTransactionsQueryDeleteAll)
}

func TestSliceDeleteAll(t *testing.T) {
	t.Run("Subplans", testSubplansSliceDeleteAll)
	t.Run("Subscriptions", testSubscriptionsSliceDeleteAll)
	t.Run("Transactions", testTransactionsSliceDeleteAll)
}

func TestExists(t *testing.T) {
	t.Run("Subplans", testSubplansExists)
	t.Run("Subscriptions", testSubscriptionsExists)
	t.Run("Transactions", testTransactionsExists)
}

func TestFind(t *testing.T) {
	t.Run("Subplans", testSubplansFind)
	t.Run("Subscriptions", testSubscriptionsFind)
	t.Run("Transactions", testTransactionsFind)
}

func TestBind(t *testing.T) {
	t.Run("Subplans", testSubplansBind)
	t.Run("Subscriptions", testSubscriptionsBind)
	t.Run("Transactions", testTransactionsBind)
}

func TestOne(t *testing.T) {
	t.Run("Subplans", testSubplansOne)
	t.Run("Subscriptions", testSubscriptionsOne)
	t.Run("Transactions", testTransactionsOne)
}

func TestAll(t *testing.T) {
	t.Run("Subplans", testSubplansAll)
	t.Run("Subscriptions", testSubscriptionsAll)
	t.Run("Transactions", testTransactionsAll)
}

func TestCount(t *testing.T) {
	t.Run("Subplans", testSubplansCount)
	t.Run("Subscriptions", testSubscriptionsCount)
	t.Run("Transactions", testTransactionsCount)
}

func TestInsert(t *testing.T) {
	t.Run("Subplans", testSubplansInsert)
	t.Run("Subplans", testSubplansInsertWhitelist)
	t.Run("Subscriptions", testSubscriptionsInsert)
	t.Run("Subscriptions", testSubscriptionsInsertWhitelist)
	t.Run("Transactions", testTransactionsInsert)
	t.Run("Transactions", testTransactionsInsertWhitelist)
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
	t.Run("Subplans", testSubplansReload)
	t.Run("Subscriptions", testSubscriptionsReload)
	t.Run("Transactions", testTransactionsReload)
}

func TestReloadAll(t *testing.T) {
	t.Run("Subplans", testSubplansReloadAll)
	t.Run("Subscriptions", testSubscriptionsReloadAll)
	t.Run("Transactions", testTransactionsReloadAll)
}

func TestSelect(t *testing.T) {
	t.Run("Subplans", testSubplansSelect)
	t.Run("Subscriptions", testSubscriptionsSelect)
	t.Run("Transactions", testTransactionsSelect)
}

func TestUpdate(t *testing.T) {
	t.Run("Subplans", testSubplansUpdate)
	t.Run("Subscriptions", testSubscriptionsUpdate)
	t.Run("Transactions", testTransactionsUpdate)
}

func TestSliceUpdateAll(t *testing.T) {
	t.Run("Subplans", testSubplansSliceUpdateAll)
	t.Run("Subscriptions", testSubscriptionsSliceUpdateAll)
	t.Run("Transactions", testTransactionsSliceUpdateAll)
}