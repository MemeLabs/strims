// Code generated by SQLBoiler 4.2.0 (https://github.com/volatiletech/sqlboiler). DO NOT EDIT.
// This file is meant to be re-generated in place and/or deleted at any time.

package models

import (
	"bytes"
	"context"
	"reflect"
	"testing"

	"github.com/volatiletech/randomize"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries"
	"github.com/volatiletech/strmangle"
)

var (
	// Relationships sometimes use the reflection helper queries.Equal/queries.Assign
	// so force a package dependency in case they don't.
	_ = queries.Equal
)

func testTransactions(t *testing.T) {
	t.Parallel()

	query := Transactions()

	if query.Query == nil {
		t.Error("expected a query, got nothing")
	}
}

func testTransactionsDelete(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	if rowsAff, err := o.Delete(ctx, tx); err != nil {
		t.Error(err)
	} else if rowsAff != 1 {
		t.Error("should only have deleted one row, but affected:", rowsAff)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 0 {
		t.Error("want zero records, got:", count)
	}
}

func testTransactionsQueryDeleteAll(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	if rowsAff, err := Transactions().DeleteAll(ctx, tx); err != nil {
		t.Error(err)
	} else if rowsAff != 1 {
		t.Error("should only have deleted one row, but affected:", rowsAff)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 0 {
		t.Error("want zero records, got:", count)
	}
}

func testTransactionsSliceDeleteAll(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	slice := TransactionSlice{o}

	if rowsAff, err := slice.DeleteAll(ctx, tx); err != nil {
		t.Error(err)
	} else if rowsAff != 1 {
		t.Error("should only have deleted one row, but affected:", rowsAff)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 0 {
		t.Error("want zero records, got:", count)
	}
}

func testTransactionsExists(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	e, err := TransactionExists(ctx, tx, o.ID)
	if err != nil {
		t.Errorf("Unable to check if Transaction exists: %s", err)
	}
	if !e {
		t.Errorf("Expected TransactionExists to return true, but got false.")
	}
}

func testTransactionsFind(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	transactionFound, err := FindTransaction(ctx, tx, o.ID)
	if err != nil {
		t.Error(err)
	}

	if transactionFound == nil {
		t.Error("want a record, got nil")
	}
}

func testTransactionsBind(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	if err = Transactions().Bind(ctx, tx, o); err != nil {
		t.Error(err)
	}
}

func testTransactionsOne(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	if x, err := Transactions().One(ctx, tx); err != nil {
		t.Error(err)
	} else if x == nil {
		t.Error("expected to get a non nil record")
	}
}

func testTransactionsAll(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	transactionOne := &Transaction{}
	transactionTwo := &Transaction{}
	if err = randomize.Struct(seed, transactionOne, transactionDBTypes, false, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}
	if err = randomize.Struct(seed, transactionTwo, transactionDBTypes, false, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = transactionOne.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}
	if err = transactionTwo.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	slice, err := Transactions().All(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if len(slice) != 2 {
		t.Error("want 2 records, got:", len(slice))
	}
}

func testTransactionsCount(t *testing.T) {
	t.Parallel()

	var err error
	seed := randomize.NewSeed()
	transactionOne := &Transaction{}
	transactionTwo := &Transaction{}
	if err = randomize.Struct(seed, transactionOne, transactionDBTypes, false, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}
	if err = randomize.Struct(seed, transactionTwo, transactionDBTypes, false, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = transactionOne.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}
	if err = transactionTwo.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 2 {
		t.Error("want 2 records, got:", count)
	}
}

func testTransactionsInsert(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 1 {
		t.Error("want one record, got:", count)
	}
}

func testTransactionsInsertWhitelist(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Whitelist(transactionColumnsWithoutDefault...)); err != nil {
		t.Error(err)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 1 {
		t.Error("want one record, got:", count)
	}
}

func testTransactionsReload(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	if err = o.Reload(ctx, tx); err != nil {
		t.Error(err)
	}
}

func testTransactionsReloadAll(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	slice := TransactionSlice{o}

	if err = slice.ReloadAll(ctx, tx); err != nil {
		t.Error(err)
	}
}

func testTransactionsSelect(t *testing.T) {
	t.Parallel()

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	slice, err := Transactions().All(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if len(slice) != 1 {
		t.Error("want one record, got:", len(slice))
	}
}

var (
	transactionDBTypes = map[string]string{`ID`: `INTEGER`, `Date`: `INTEGER`, `Subject`: `TEXT`, `Note`: `TEXT`, `Currency`: `TEXT`, `Amount`: `REAL`, `Ending`: `REAL`, `Available`: `REAL`, `Service`: `TEXT`}
	_                  = bytes.MinRead
)

func testTransactionsUpdate(t *testing.T) {
	t.Parallel()

	if 0 == len(transactionPrimaryKeyColumns) {
		t.Skip("Skipping table with no primary key columns")
	}
	if len(transactionAllColumns) == len(transactionPrimaryKeyColumns) {
		t.Skip("Skipping table with only primary key columns")
	}

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 1 {
		t.Error("want one record, got:", count)
	}

	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionPrimaryKeyColumns...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	if rowsAff, err := o.Update(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	} else if rowsAff != 1 {
		t.Error("should only affect one row but affected", rowsAff)
	}
}

func testTransactionsSliceUpdateAll(t *testing.T) {
	t.Parallel()

	if len(transactionAllColumns) == len(transactionPrimaryKeyColumns) {
		t.Skip("Skipping table with only primary key columns")
	}

	seed := randomize.NewSeed()
	var err error
	o := &Transaction{}
	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionColumnsWithDefault...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	ctx := context.Background()
	tx := MustTx(boil.BeginTx(ctx, nil))
	defer func() { _ = tx.Rollback() }()
	if err = o.Insert(ctx, tx, boil.Infer()); err != nil {
		t.Error(err)
	}

	count, err := Transactions().Count(ctx, tx)
	if err != nil {
		t.Error(err)
	}

	if count != 1 {
		t.Error("want one record, got:", count)
	}

	if err = randomize.Struct(seed, o, transactionDBTypes, true, transactionPrimaryKeyColumns...); err != nil {
		t.Errorf("Unable to randomize Transaction struct: %s", err)
	}

	// Remove Primary keys and unique columns from what we plan to update
	var fields []string
	if strmangle.StringSliceMatch(transactionAllColumns, transactionPrimaryKeyColumns) {
		fields = transactionAllColumns
	} else {
		fields = strmangle.SetComplement(
			transactionAllColumns,
			transactionPrimaryKeyColumns,
		)
	}

	value := reflect.Indirect(reflect.ValueOf(o))
	typ := reflect.TypeOf(o).Elem()
	n := typ.NumField()

	updateMap := M{}
	for _, col := range fields {
		for i := 0; i < n; i++ {
			f := typ.Field(i)
			if f.Tag.Get("boil") == col {
				updateMap[col] = value.Field(i).Interface()
			}
		}
	}

	slice := TransactionSlice{o}
	if rowsAff, err := slice.UpdateAll(ctx, tx, updateMap); err != nil {
		t.Error(err)
	} else if rowsAff != 1 {
		t.Error("wanted one record updated but got", rowsAff)
	}
}
