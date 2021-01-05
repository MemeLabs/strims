// Code generated by SQLBoiler 4.2.0 (https://github.com/volatiletech/sqlboiler). DO NOT EDIT.
// This file is meant to be re-generated in place and/or deleted at any time.

package models

import (
	"context"
	"database/sql"
	"fmt"
	"reflect"
	"strings"
	"sync"
	"time"

	"github.com/friendsofgo/errors"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"github.com/volatiletech/sqlboiler/v4/queries/qmhelper"
	"github.com/volatiletech/strmangle"
)

// Subscription is an object representing the database table.
type Subscription struct {
	ID        null.Int64 `boil:"id" json:"id,omitempty" toml:"id" yaml:"id,omitempty"`
	SubPlanID string     `boil:"sub_plan_id" json:"subPlanID" toml:"subPlanID" yaml:"subPlanID"`
	StartDate int64      `boil:"start_date" json:"startDate" toml:"startDate" yaml:"startDate"`
	EndDate   int64      `boil:"end_date" json:"endDate" toml:"endDate" yaml:"endDate"`

	R *subscriptionR `boil:"-" json:"-" toml:"-" yaml:"-"`
	L subscriptionL  `boil:"-" json:"-" toml:"-" yaml:"-"`
}

var SubscriptionColumns = struct {
	ID        string
	SubPlanID string
	StartDate string
	EndDate   string
}{
	ID:        "id",
	SubPlanID: "sub_plan_id",
	StartDate: "start_date",
	EndDate:   "end_date",
}

// Generated where

var SubscriptionWhere = struct {
	ID        whereHelpernull_Int64
	SubPlanID whereHelperstring
	StartDate whereHelperint64
	EndDate   whereHelperint64
}{
	ID:        whereHelpernull_Int64{field: "\"subscriptions\".\"id\""},
	SubPlanID: whereHelperstring{field: "\"subscriptions\".\"sub_plan_id\""},
	StartDate: whereHelperint64{field: "\"subscriptions\".\"start_date\""},
	EndDate:   whereHelperint64{field: "\"subscriptions\".\"end_date\""},
}

// SubscriptionRels is where relationship names are stored.
var SubscriptionRels = struct {
}{}

// subscriptionR is where relationships are stored.
type subscriptionR struct {
}

// NewStruct creates a new relationship struct
func (*subscriptionR) NewStruct() *subscriptionR {
	return &subscriptionR{}
}

// subscriptionL is where Load methods for each relationship are stored.
type subscriptionL struct{}

var (
	subscriptionAllColumns            = []string{"id", "sub_plan_id", "start_date", "end_date"}
	subscriptionColumnsWithoutDefault = []string{}
	subscriptionColumnsWithDefault    = []string{"id", "sub_plan_id", "start_date", "end_date"}
	subscriptionPrimaryKeyColumns     = []string{"id"}
)

type (
	// SubscriptionSlice is an alias for a slice of pointers to Subscription.
	// This should generally be used opposed to []Subscription.
	SubscriptionSlice []*Subscription

	subscriptionQuery struct {
		*queries.Query
	}
)

// Cache for insert, update and upsert
var (
	subscriptionType                 = reflect.TypeOf(&Subscription{})
	subscriptionMapping              = queries.MakeStructMapping(subscriptionType)
	subscriptionPrimaryKeyMapping, _ = queries.BindMapping(subscriptionType, subscriptionMapping, subscriptionPrimaryKeyColumns)
	subscriptionInsertCacheMut       sync.RWMutex
	subscriptionInsertCache          = make(map[string]insertCache)
	subscriptionUpdateCacheMut       sync.RWMutex
	subscriptionUpdateCache          = make(map[string]updateCache)
	subscriptionUpsertCacheMut       sync.RWMutex
	subscriptionUpsertCache          = make(map[string]insertCache)
)

var (
	// Force time package dependency for automated UpdatedAt/CreatedAt.
	_ = time.Second
	// Force qmhelper dependency for where clause generation (which doesn't
	// always happen)
	_ = qmhelper.Where
)

// OneG returns a single subscription record from the query using the global executor.
func (q subscriptionQuery) OneG(ctx context.Context) (*Subscription, error) {
	return q.One(ctx, boil.GetContextDB())
}

// One returns a single subscription record from the query.
func (q subscriptionQuery) One(ctx context.Context, exec boil.ContextExecutor) (*Subscription, error) {
	o := &Subscription{}

	queries.SetLimit(q.Query, 1)

	err := q.Bind(ctx, exec, o)
	if err != nil {
		if errors.Cause(err) == sql.ErrNoRows {
			return nil, sql.ErrNoRows
		}
		return nil, errors.Wrap(err, "models: failed to execute a one query for subscriptions")
	}

	return o, nil
}

// AllG returns all Subscription records from the query using the global executor.
func (q subscriptionQuery) AllG(ctx context.Context) (SubscriptionSlice, error) {
	return q.All(ctx, boil.GetContextDB())
}

// All returns all Subscription records from the query.
func (q subscriptionQuery) All(ctx context.Context, exec boil.ContextExecutor) (SubscriptionSlice, error) {
	var o []*Subscription

	err := q.Bind(ctx, exec, &o)
	if err != nil {
		return nil, errors.Wrap(err, "models: failed to assign all query results to Subscription slice")
	}

	return o, nil
}

// CountG returns the count of all Subscription records in the query, and panics on error.
func (q subscriptionQuery) CountG(ctx context.Context) (int64, error) {
	return q.Count(ctx, boil.GetContextDB())
}

// Count returns the count of all Subscription records in the query.
func (q subscriptionQuery) Count(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	var count int64

	queries.SetSelect(q.Query, nil)
	queries.SetCount(q.Query)

	err := q.Query.QueryRowContext(ctx, exec).Scan(&count)
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to count subscriptions rows")
	}

	return count, nil
}

// ExistsG checks if the row exists in the table, and panics on error.
func (q subscriptionQuery) ExistsG(ctx context.Context) (bool, error) {
	return q.Exists(ctx, boil.GetContextDB())
}

// Exists checks if the row exists in the table.
func (q subscriptionQuery) Exists(ctx context.Context, exec boil.ContextExecutor) (bool, error) {
	var count int64

	queries.SetSelect(q.Query, nil)
	queries.SetCount(q.Query)
	queries.SetLimit(q.Query, 1)

	err := q.Query.QueryRowContext(ctx, exec).Scan(&count)
	if err != nil {
		return false, errors.Wrap(err, "models: failed to check if subscriptions exists")
	}

	return count > 0, nil
}

// Subscriptions retrieves all the records using an executor.
func Subscriptions(mods ...qm.QueryMod) subscriptionQuery {
	mods = append(mods, qm.From("\"subscriptions\""))
	return subscriptionQuery{NewQuery(mods...)}
}

// FindSubscriptionG retrieves a single record by ID.
func FindSubscriptionG(ctx context.Context, iD null.Int64, selectCols ...string) (*Subscription, error) {
	return FindSubscription(ctx, boil.GetContextDB(), iD, selectCols...)
}

// FindSubscription retrieves a single record by ID with an executor.
// If selectCols is empty Find will return all columns.
func FindSubscription(ctx context.Context, exec boil.ContextExecutor, iD null.Int64, selectCols ...string) (*Subscription, error) {
	subscriptionObj := &Subscription{}

	sel := "*"
	if len(selectCols) > 0 {
		sel = strings.Join(strmangle.IdentQuoteSlice(dialect.LQ, dialect.RQ, selectCols), ",")
	}
	query := fmt.Sprintf(
		"select %s from \"subscriptions\" where \"id\"=?", sel,
	)

	q := queries.Raw(query, iD)

	err := q.Bind(ctx, exec, subscriptionObj)
	if err != nil {
		if errors.Cause(err) == sql.ErrNoRows {
			return nil, sql.ErrNoRows
		}
		return nil, errors.Wrap(err, "models: unable to select from subscriptions")
	}

	return subscriptionObj, nil
}

// InsertG a single record. See Insert for whitelist behavior description.
func (o *Subscription) InsertG(ctx context.Context, columns boil.Columns) error {
	return o.Insert(ctx, boil.GetContextDB(), columns)
}

// Insert a single record using an executor.
// See boil.Columns.InsertColumnSet documentation to understand column list inference for inserts.
func (o *Subscription) Insert(ctx context.Context, exec boil.ContextExecutor, columns boil.Columns) error {
	if o == nil {
		return errors.New("models: no subscriptions provided for insertion")
	}

	var err error

	nzDefaults := queries.NonZeroDefaultSet(subscriptionColumnsWithDefault, o)

	key := makeCacheKey(columns, nzDefaults)
	subscriptionInsertCacheMut.RLock()
	cache, cached := subscriptionInsertCache[key]
	subscriptionInsertCacheMut.RUnlock()

	if !cached {
		wl, returnColumns := columns.InsertColumnSet(
			subscriptionAllColumns,
			subscriptionColumnsWithDefault,
			subscriptionColumnsWithoutDefault,
			nzDefaults,
		)

		cache.valueMapping, err = queries.BindMapping(subscriptionType, subscriptionMapping, wl)
		if err != nil {
			return err
		}
		cache.retMapping, err = queries.BindMapping(subscriptionType, subscriptionMapping, returnColumns)
		if err != nil {
			return err
		}
		if len(wl) != 0 {
			cache.query = fmt.Sprintf("INSERT INTO \"subscriptions\" (\"%s\") %%sVALUES (%s)%%s", strings.Join(wl, "\",\""), strmangle.Placeholders(dialect.UseIndexPlaceholders, len(wl), 1, 1))
		} else {
			cache.query = "INSERT INTO \"subscriptions\" %sDEFAULT VALUES%s"
		}

		var queryOutput, queryReturning string

		if len(cache.retMapping) != 0 {
			cache.retQuery = fmt.Sprintf("SELECT \"%s\" FROM \"subscriptions\" WHERE %s", strings.Join(returnColumns, "\",\""), strmangle.WhereClause("\"", "\"", 0, subscriptionPrimaryKeyColumns))
		}

		cache.query = fmt.Sprintf(cache.query, queryOutput, queryReturning)
	}

	value := reflect.Indirect(reflect.ValueOf(o))
	vals := queries.ValuesFromMapping(value, cache.valueMapping)

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, cache.query)
		fmt.Fprintln(writer, vals)
	}
	_, err = exec.ExecContext(ctx, cache.query, vals...)

	if err != nil {
		return errors.Wrap(err, "models: unable to insert into subscriptions")
	}

	var identifierCols []interface{}

	if len(cache.retMapping) == 0 {
		goto CacheNoHooks
	}

	identifierCols = []interface{}{
		o.ID,
	}

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, cache.retQuery)
		fmt.Fprintln(writer, identifierCols...)
	}
	err = exec.QueryRowContext(ctx, cache.retQuery, identifierCols...).Scan(queries.PtrsFromMapping(value, cache.retMapping)...)
	if err != nil {
		return errors.Wrap(err, "models: unable to populate default values for subscriptions")
	}

CacheNoHooks:
	if !cached {
		subscriptionInsertCacheMut.Lock()
		subscriptionInsertCache[key] = cache
		subscriptionInsertCacheMut.Unlock()
	}

	return nil
}

// UpdateG a single Subscription record using the global executor.
// See Update for more documentation.
func (o *Subscription) UpdateG(ctx context.Context, columns boil.Columns) (int64, error) {
	return o.Update(ctx, boil.GetContextDB(), columns)
}

// Update uses an executor to update the Subscription.
// See boil.Columns.UpdateColumnSet documentation to understand column list inference for updates.
// Update does not automatically update the record in case of default values. Use .Reload() to refresh the records.
func (o *Subscription) Update(ctx context.Context, exec boil.ContextExecutor, columns boil.Columns) (int64, error) {
	var err error
	key := makeCacheKey(columns, nil)
	subscriptionUpdateCacheMut.RLock()
	cache, cached := subscriptionUpdateCache[key]
	subscriptionUpdateCacheMut.RUnlock()

	if !cached {
		wl := columns.UpdateColumnSet(
			subscriptionAllColumns,
			subscriptionPrimaryKeyColumns,
		)

		if !columns.IsWhitelist() {
			wl = strmangle.SetComplement(wl, []string{"created_at"})
		}
		if len(wl) == 0 {
			return 0, errors.New("models: unable to update subscriptions, could not build whitelist")
		}

		cache.query = fmt.Sprintf("UPDATE \"subscriptions\" SET %s WHERE %s",
			strmangle.SetParamNames("\"", "\"", 0, wl),
			strmangle.WhereClause("\"", "\"", 0, subscriptionPrimaryKeyColumns),
		)
		cache.valueMapping, err = queries.BindMapping(subscriptionType, subscriptionMapping, append(wl, subscriptionPrimaryKeyColumns...))
		if err != nil {
			return 0, err
		}
	}

	values := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(o)), cache.valueMapping)

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, cache.query)
		fmt.Fprintln(writer, values)
	}
	var result sql.Result
	result, err = exec.ExecContext(ctx, cache.query, values...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to update subscriptions row")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by update for subscriptions")
	}

	if !cached {
		subscriptionUpdateCacheMut.Lock()
		subscriptionUpdateCache[key] = cache
		subscriptionUpdateCacheMut.Unlock()
	}

	return rowsAff, nil
}

// UpdateAllG updates all rows with the specified column values.
func (q subscriptionQuery) UpdateAllG(ctx context.Context, cols M) (int64, error) {
	return q.UpdateAll(ctx, boil.GetContextDB(), cols)
}

// UpdateAll updates all rows with the specified column values.
func (q subscriptionQuery) UpdateAll(ctx context.Context, exec boil.ContextExecutor, cols M) (int64, error) {
	queries.SetUpdate(q.Query, cols)

	result, err := q.Query.ExecContext(ctx, exec)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to update all for subscriptions")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to retrieve rows affected for subscriptions")
	}

	return rowsAff, nil
}

// UpdateAllG updates all rows with the specified column values.
func (o SubscriptionSlice) UpdateAllG(ctx context.Context, cols M) (int64, error) {
	return o.UpdateAll(ctx, boil.GetContextDB(), cols)
}

// UpdateAll updates all rows with the specified column values, using an executor.
func (o SubscriptionSlice) UpdateAll(ctx context.Context, exec boil.ContextExecutor, cols M) (int64, error) {
	ln := int64(len(o))
	if ln == 0 {
		return 0, nil
	}

	if len(cols) == 0 {
		return 0, errors.New("models: update all requires at least one column argument")
	}

	colNames := make([]string, len(cols))
	args := make([]interface{}, len(cols))

	i := 0
	for name, value := range cols {
		colNames[i] = name
		args[i] = value
		i++
	}

	// Append all of the primary key values for each column
	for _, obj := range o {
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), subscriptionPrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := fmt.Sprintf("UPDATE \"subscriptions\" SET %s WHERE %s",
		strmangle.SetParamNames("\"", "\"", 0, colNames),
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), 0, subscriptionPrimaryKeyColumns, len(o)))

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args...)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to update all in subscription slice")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to retrieve rows affected all in update all subscription")
	}
	return rowsAff, nil
}

// DeleteG deletes a single Subscription record.
// DeleteG will match against the primary key column to find the record to delete.
func (o *Subscription) DeleteG(ctx context.Context) (int64, error) {
	return o.Delete(ctx, boil.GetContextDB())
}

// Delete deletes a single Subscription record with an executor.
// Delete will match against the primary key column to find the record to delete.
func (o *Subscription) Delete(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if o == nil {
		return 0, errors.New("models: no Subscription provided for delete")
	}

	args := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(o)), subscriptionPrimaryKeyMapping)
	sql := "DELETE FROM \"subscriptions\" WHERE \"id\"=?"

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args...)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete from subscriptions")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by delete for subscriptions")
	}

	return rowsAff, nil
}

func (q subscriptionQuery) DeleteAllG(ctx context.Context) (int64, error) {
	return q.DeleteAll(ctx, boil.GetContextDB())
}

// DeleteAll deletes all matching rows.
func (q subscriptionQuery) DeleteAll(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if q.Query == nil {
		return 0, errors.New("models: no subscriptionQuery provided for delete all")
	}

	queries.SetDelete(q.Query)

	result, err := q.Query.ExecContext(ctx, exec)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete all from subscriptions")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by deleteall for subscriptions")
	}

	return rowsAff, nil
}

// DeleteAllG deletes all rows in the slice.
func (o SubscriptionSlice) DeleteAllG(ctx context.Context) (int64, error) {
	return o.DeleteAll(ctx, boil.GetContextDB())
}

// DeleteAll deletes all rows in the slice, using an executor.
func (o SubscriptionSlice) DeleteAll(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if len(o) == 0 {
		return 0, nil
	}

	var args []interface{}
	for _, obj := range o {
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), subscriptionPrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := "DELETE FROM \"subscriptions\" WHERE " +
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), 0, subscriptionPrimaryKeyColumns, len(o))

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete all from subscription slice")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by deleteall for subscriptions")
	}

	return rowsAff, nil
}

// ReloadG refetches the object from the database using the primary keys.
func (o *Subscription) ReloadG(ctx context.Context) error {
	if o == nil {
		return errors.New("models: no Subscription provided for reload")
	}

	return o.Reload(ctx, boil.GetContextDB())
}

// Reload refetches the object from the database
// using the primary keys with an executor.
func (o *Subscription) Reload(ctx context.Context, exec boil.ContextExecutor) error {
	ret, err := FindSubscription(ctx, exec, o.ID)
	if err != nil {
		return err
	}

	*o = *ret
	return nil
}

// ReloadAllG refetches every row with matching primary key column values
// and overwrites the original object slice with the newly updated slice.
func (o *SubscriptionSlice) ReloadAllG(ctx context.Context) error {
	if o == nil {
		return errors.New("models: empty SubscriptionSlice provided for reload all")
	}

	return o.ReloadAll(ctx, boil.GetContextDB())
}

// ReloadAll refetches every row with matching primary key column values
// and overwrites the original object slice with the newly updated slice.
func (o *SubscriptionSlice) ReloadAll(ctx context.Context, exec boil.ContextExecutor) error {
	if o == nil || len(*o) == 0 {
		return nil
	}

	slice := SubscriptionSlice{}
	var args []interface{}
	for _, obj := range *o {
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), subscriptionPrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := "SELECT \"subscriptions\".* FROM \"subscriptions\" WHERE " +
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), 0, subscriptionPrimaryKeyColumns, len(*o))

	q := queries.Raw(sql, args...)

	err := q.Bind(ctx, exec, &slice)
	if err != nil {
		return errors.Wrap(err, "models: unable to reload all in SubscriptionSlice")
	}

	*o = slice

	return nil
}

// SubscriptionExistsG checks if the Subscription row exists.
func SubscriptionExistsG(ctx context.Context, iD null.Int64) (bool, error) {
	return SubscriptionExists(ctx, boil.GetContextDB(), iD)
}

// SubscriptionExists checks if the Subscription row exists.
func SubscriptionExists(ctx context.Context, exec boil.ContextExecutor, iD null.Int64) (bool, error) {
	var exists bool
	sql := "select exists(select 1 from \"subscriptions\" where \"id\"=? limit 1)"

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, iD)
	}
	row := exec.QueryRowContext(ctx, sql, iD)

	err := row.Scan(&exists)
	if err != nil {
		return false, errors.Wrap(err, "models: unable to check if subscriptions exists")
	}

	return exists, nil
}
