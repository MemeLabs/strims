// Code generated by SQLBoiler 4.2.0 (https://github.com/volatiletech/sqlboiler). DO NOT EDIT.
// This file is meant to be re-generated in place and/or deleted at any time.

package models

import (
	"context"
	"database/sql"
	"fmt"
	"reflect"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/friendsofgo/errors"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"github.com/volatiletech/sqlboiler/v4/queries/qmhelper"
	"github.com/volatiletech/strmangle"
)

// Usage is an object representing the database table.
type Usage struct {
	ID         int64   `boil:"id" json:"id" toml:"id" yaml:"id"`
	NodeID     int64   `boil:"node_id" json:"nodeID" toml:"nodeID" yaml:"nodeID"`
	Time       int64   `boil:"time" json:"time" toml:"time" yaml:"time"`
	Mem        float64 `boil:"mem" json:"mem" toml:"mem" yaml:"mem"`
	CPU        float64 `boil:"cpu" json:"cpu" toml:"cpu" yaml:"cpu"`
	NetworkIn  float64 `boil:"network_in" json:"networkIn" toml:"networkIn" yaml:"networkIn"`
	NetworkOut float64 `boil:"network_out" json:"networkOut" toml:"networkOut" yaml:"networkOut"`

	R *usageR `boil:"-" json:"-" toml:"-" yaml:"-"`
	L usageL  `boil:"-" json:"-" toml:"-" yaml:"-"`
}

var UsageColumns = struct {
	ID         string
	NodeID     string
	Time       string
	Mem        string
	CPU        string
	NetworkIn  string
	NetworkOut string
}{
	ID:         "id",
	NodeID:     "node_id",
	Time:       "time",
	Mem:        "mem",
	CPU:        "cpu",
	NetworkIn:  "network_in",
	NetworkOut: "network_out",
}

// Generated where

type whereHelperfloat64 struct{ field string }

func (w whereHelperfloat64) EQ(x float64) qm.QueryMod { return qmhelper.Where(w.field, qmhelper.EQ, x) }
func (w whereHelperfloat64) NEQ(x float64) qm.QueryMod {
	return qmhelper.Where(w.field, qmhelper.NEQ, x)
}
func (w whereHelperfloat64) LT(x float64) qm.QueryMod { return qmhelper.Where(w.field, qmhelper.LT, x) }
func (w whereHelperfloat64) LTE(x float64) qm.QueryMod {
	return qmhelper.Where(w.field, qmhelper.LTE, x)
}
func (w whereHelperfloat64) GT(x float64) qm.QueryMod { return qmhelper.Where(w.field, qmhelper.GT, x) }
func (w whereHelperfloat64) GTE(x float64) qm.QueryMod {
	return qmhelper.Where(w.field, qmhelper.GTE, x)
}
func (w whereHelperfloat64) IN(slice []float64) qm.QueryMod {
	values := make([]interface{}, 0, len(slice))
	for _, value := range slice {
		values = append(values, value)
	}
	return qm.WhereIn(fmt.Sprintf("%s IN ?", w.field), values...)
}
func (w whereHelperfloat64) NIN(slice []float64) qm.QueryMod {
	values := make([]interface{}, 0, len(slice))
	for _, value := range slice {
		values = append(values, value)
	}
	return qm.WhereNotIn(fmt.Sprintf("%s NOT IN ?", w.field), values...)
}

var UsageWhere = struct {
	ID         whereHelperint64
	NodeID     whereHelperint64
	Time       whereHelperint64
	Mem        whereHelperfloat64
	CPU        whereHelperfloat64
	NetworkIn  whereHelperfloat64
	NetworkOut whereHelperfloat64
}{
	ID:         whereHelperint64{field: "\"usage\".\"id\""},
	NodeID:     whereHelperint64{field: "\"usage\".\"node_id\""},
	Time:       whereHelperint64{field: "\"usage\".\"time\""},
	Mem:        whereHelperfloat64{field: "\"usage\".\"mem\""},
	CPU:        whereHelperfloat64{field: "\"usage\".\"cpu\""},
	NetworkIn:  whereHelperfloat64{field: "\"usage\".\"network_in\""},
	NetworkOut: whereHelperfloat64{field: "\"usage\".\"network_out\""},
}

// UsageRels is where relationship names are stored.
var UsageRels = struct {
}{}

// usageR is where relationships are stored.
type usageR struct {
}

// NewStruct creates a new relationship struct
func (*usageR) NewStruct() *usageR {
	return &usageR{}
}

// usageL is where Load methods for each relationship are stored.
type usageL struct{}

var (
	usageAllColumns            = []string{"id", "node_id", "time", "mem", "cpu", "network_in", "network_out"}
	usageColumnsWithoutDefault = []string{"node_id", "time", "mem", "cpu", "network_in", "network_out"}
	usageColumnsWithDefault    = []string{"id"}
	usagePrimaryKeyColumns     = []string{"id"}
)

type (
	// UsageSlice is an alias for a slice of pointers to Usage.
	// This should generally be used opposed to []Usage.
	UsageSlice []*Usage

	usageQuery struct {
		*queries.Query
	}
)

// Cache for insert, update and upsert
var (
	usageType                 = reflect.TypeOf(&Usage{})
	usageMapping              = queries.MakeStructMapping(usageType)
	usagePrimaryKeyMapping, _ = queries.BindMapping(usageType, usageMapping, usagePrimaryKeyColumns)
	usageInsertCacheMut       sync.RWMutex
	usageInsertCache          = make(map[string]insertCache)
	usageUpdateCacheMut       sync.RWMutex
	usageUpdateCache          = make(map[string]updateCache)
	usageUpsertCacheMut       sync.RWMutex
	usageUpsertCache          = make(map[string]insertCache)
)

var (
	// Force time package dependency for automated UpdatedAt/CreatedAt.
	_ = time.Second
	// Force qmhelper dependency for where clause generation (which doesn't
	// always happen)
	_ = qmhelper.Where
)

// OneG returns a single usage record from the query using the global executor.
func (q usageQuery) OneG(ctx context.Context) (*Usage, error) {
	return q.One(ctx, boil.GetContextDB())
}

// One returns a single usage record from the query.
func (q usageQuery) One(ctx context.Context, exec boil.ContextExecutor) (*Usage, error) {
	o := &Usage{}

	queries.SetLimit(q.Query, 1)

	err := q.Bind(ctx, exec, o)
	if err != nil {
		if errors.Cause(err) == sql.ErrNoRows {
			return nil, sql.ErrNoRows
		}
		return nil, errors.Wrap(err, "models: failed to execute a one query for usage")
	}

	return o, nil
}

// AllG returns all Usage records from the query using the global executor.
func (q usageQuery) AllG(ctx context.Context) (UsageSlice, error) {
	return q.All(ctx, boil.GetContextDB())
}

// All returns all Usage records from the query.
func (q usageQuery) All(ctx context.Context, exec boil.ContextExecutor) (UsageSlice, error) {
	var o []*Usage

	err := q.Bind(ctx, exec, &o)
	if err != nil {
		return nil, errors.Wrap(err, "models: failed to assign all query results to Usage slice")
	}

	return o, nil
}

// CountG returns the count of all Usage records in the query, and panics on error.
func (q usageQuery) CountG(ctx context.Context) (int64, error) {
	return q.Count(ctx, boil.GetContextDB())
}

// Count returns the count of all Usage records in the query.
func (q usageQuery) Count(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	var count int64

	queries.SetSelect(q.Query, nil)
	queries.SetCount(q.Query)

	err := q.Query.QueryRowContext(ctx, exec).Scan(&count)
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to count usage rows")
	}

	return count, nil
}

// ExistsG checks if the row exists in the table, and panics on error.
func (q usageQuery) ExistsG(ctx context.Context) (bool, error) {
	return q.Exists(ctx, boil.GetContextDB())
}

// Exists checks if the row exists in the table.
func (q usageQuery) Exists(ctx context.Context, exec boil.ContextExecutor) (bool, error) {
	var count int64

	queries.SetSelect(q.Query, nil)
	queries.SetCount(q.Query)
	queries.SetLimit(q.Query, 1)

	err := q.Query.QueryRowContext(ctx, exec).Scan(&count)
	if err != nil {
		return false, errors.Wrap(err, "models: failed to check if usage exists")
	}

	return count > 0, nil
}

// Usages retrieves all the records using an executor.
func Usages(mods ...qm.QueryMod) usageQuery {
	mods = append(mods, qm.From("\"usage\""))
	return usageQuery{NewQuery(mods...)}
}

// FindUsageG retrieves a single record by ID.
func FindUsageG(ctx context.Context, iD int64, selectCols ...string) (*Usage, error) {
	return FindUsage(ctx, boil.GetContextDB(), iD, selectCols...)
}

// FindUsage retrieves a single record by ID with an executor.
// If selectCols is empty Find will return all columns.
func FindUsage(ctx context.Context, exec boil.ContextExecutor, iD int64, selectCols ...string) (*Usage, error) {
	usageObj := &Usage{}

	sel := "*"
	if len(selectCols) > 0 {
		sel = strings.Join(strmangle.IdentQuoteSlice(dialect.LQ, dialect.RQ, selectCols), ",")
	}
	query := fmt.Sprintf(
		"select %s from \"usage\" where \"id\"=$1", sel,
	)

	q := queries.Raw(query, iD)

	err := q.Bind(ctx, exec, usageObj)
	if err != nil {
		if errors.Cause(err) == sql.ErrNoRows {
			return nil, sql.ErrNoRows
		}
		return nil, errors.Wrap(err, "models: unable to select from usage")
	}

	return usageObj, nil
}

// InsertG a single record. See Insert for whitelist behavior description.
func (o *Usage) InsertG(ctx context.Context, columns boil.Columns) error {
	return o.Insert(ctx, boil.GetContextDB(), columns)
}

// Insert a single record using an executor.
// See boil.Columns.InsertColumnSet documentation to understand column list inference for inserts.
func (o *Usage) Insert(ctx context.Context, exec boil.ContextExecutor, columns boil.Columns) error {
	if o == nil {
		return errors.New("models: no usage provided for insertion")
	}

	var err error

	nzDefaults := queries.NonZeroDefaultSet(usageColumnsWithDefault, o)

	key := makeCacheKey(columns, nzDefaults)
	usageInsertCacheMut.RLock()
	cache, cached := usageInsertCache[key]
	usageInsertCacheMut.RUnlock()

	if !cached {
		wl, returnColumns := columns.InsertColumnSet(
			usageAllColumns,
			usageColumnsWithDefault,
			usageColumnsWithoutDefault,
			nzDefaults,
		)

		cache.valueMapping, err = queries.BindMapping(usageType, usageMapping, wl)
		if err != nil {
			return err
		}
		cache.retMapping, err = queries.BindMapping(usageType, usageMapping, returnColumns)
		if err != nil {
			return err
		}
		if len(wl) != 0 {
			cache.query = fmt.Sprintf("INSERT INTO \"usage\" (\"%s\") %%sVALUES (%s)%%s", strings.Join(wl, "\",\""), strmangle.Placeholders(dialect.UseIndexPlaceholders, len(wl), 1, 1))
		} else {
			cache.query = "INSERT INTO \"usage\" %sDEFAULT VALUES%s"
		}

		var queryOutput, queryReturning string

		if len(cache.retMapping) != 0 {
			queryReturning = fmt.Sprintf(" RETURNING \"%s\"", strings.Join(returnColumns, "\",\""))
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

	if len(cache.retMapping) != 0 {
		err = exec.QueryRowContext(ctx, cache.query, vals...).Scan(queries.PtrsFromMapping(value, cache.retMapping)...)
	} else {
		_, err = exec.ExecContext(ctx, cache.query, vals...)
	}

	if err != nil {
		return errors.Wrap(err, "models: unable to insert into usage")
	}

	if !cached {
		usageInsertCacheMut.Lock()
		usageInsertCache[key] = cache
		usageInsertCacheMut.Unlock()
	}

	return nil
}

// UpdateG a single Usage record using the global executor.
// See Update for more documentation.
func (o *Usage) UpdateG(ctx context.Context, columns boil.Columns) (int64, error) {
	return o.Update(ctx, boil.GetContextDB(), columns)
}

// Update uses an executor to update the Usage.
// See boil.Columns.UpdateColumnSet documentation to understand column list inference for updates.
// Update does not automatically update the record in case of default values. Use .Reload() to refresh the records.
func (o *Usage) Update(ctx context.Context, exec boil.ContextExecutor, columns boil.Columns) (int64, error) {
	var err error
	key := makeCacheKey(columns, nil)
	usageUpdateCacheMut.RLock()
	cache, cached := usageUpdateCache[key]
	usageUpdateCacheMut.RUnlock()

	if !cached {
		wl := columns.UpdateColumnSet(
			usageAllColumns,
			usagePrimaryKeyColumns,
		)

		if !columns.IsWhitelist() {
			wl = strmangle.SetComplement(wl, []string{"created_at"})
		}
		if len(wl) == 0 {
			return 0, errors.New("models: unable to update usage, could not build whitelist")
		}

		cache.query = fmt.Sprintf("UPDATE \"usage\" SET %s WHERE %s",
			strmangle.SetParamNames("\"", "\"", 1, wl),
			strmangle.WhereClause("\"", "\"", len(wl)+1, usagePrimaryKeyColumns),
		)
		cache.valueMapping, err = queries.BindMapping(usageType, usageMapping, append(wl, usagePrimaryKeyColumns...))
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
		return 0, errors.Wrap(err, "models: unable to update usage row")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by update for usage")
	}

	if !cached {
		usageUpdateCacheMut.Lock()
		usageUpdateCache[key] = cache
		usageUpdateCacheMut.Unlock()
	}

	return rowsAff, nil
}

// UpdateAllG updates all rows with the specified column values.
func (q usageQuery) UpdateAllG(ctx context.Context, cols M) (int64, error) {
	return q.UpdateAll(ctx, boil.GetContextDB(), cols)
}

// UpdateAll updates all rows with the specified column values.
func (q usageQuery) UpdateAll(ctx context.Context, exec boil.ContextExecutor, cols M) (int64, error) {
	queries.SetUpdate(q.Query, cols)

	result, err := q.Query.ExecContext(ctx, exec)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to update all for usage")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to retrieve rows affected for usage")
	}

	return rowsAff, nil
}

// UpdateAllG updates all rows with the specified column values.
func (o UsageSlice) UpdateAllG(ctx context.Context, cols M) (int64, error) {
	return o.UpdateAll(ctx, boil.GetContextDB(), cols)
}

// UpdateAll updates all rows with the specified column values, using an executor.
func (o UsageSlice) UpdateAll(ctx context.Context, exec boil.ContextExecutor, cols M) (int64, error) {
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
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), usagePrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := fmt.Sprintf("UPDATE \"usage\" SET %s WHERE %s",
		strmangle.SetParamNames("\"", "\"", 1, colNames),
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), len(colNames)+1, usagePrimaryKeyColumns, len(o)))

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args...)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to update all in usage slice")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to retrieve rows affected all in update all usage")
	}
	return rowsAff, nil
}

// UpsertG attempts an insert, and does an update or ignore on conflict.
func (o *Usage) UpsertG(ctx context.Context, updateOnConflict bool, conflictColumns []string, updateColumns, insertColumns boil.Columns) error {
	return o.Upsert(ctx, boil.GetContextDB(), updateOnConflict, conflictColumns, updateColumns, insertColumns)
}

// Upsert attempts an insert using an executor, and does an update or ignore on conflict.
// See boil.Columns documentation for how to properly use updateColumns and insertColumns.
func (o *Usage) Upsert(ctx context.Context, exec boil.ContextExecutor, updateOnConflict bool, conflictColumns []string, updateColumns, insertColumns boil.Columns) error {
	if o == nil {
		return errors.New("models: no usage provided for upsert")
	}

	nzDefaults := queries.NonZeroDefaultSet(usageColumnsWithDefault, o)

	// Build cache key in-line uglily - mysql vs psql problems
	buf := strmangle.GetBuffer()
	if updateOnConflict {
		buf.WriteByte('t')
	} else {
		buf.WriteByte('f')
	}
	buf.WriteByte('.')
	for _, c := range conflictColumns {
		buf.WriteString(c)
	}
	buf.WriteByte('.')
	buf.WriteString(strconv.Itoa(updateColumns.Kind))
	for _, c := range updateColumns.Cols {
		buf.WriteString(c)
	}
	buf.WriteByte('.')
	buf.WriteString(strconv.Itoa(insertColumns.Kind))
	for _, c := range insertColumns.Cols {
		buf.WriteString(c)
	}
	buf.WriteByte('.')
	for _, c := range nzDefaults {
		buf.WriteString(c)
	}
	key := buf.String()
	strmangle.PutBuffer(buf)

	usageUpsertCacheMut.RLock()
	cache, cached := usageUpsertCache[key]
	usageUpsertCacheMut.RUnlock()

	var err error

	if !cached {
		insert, ret := insertColumns.InsertColumnSet(
			usageAllColumns,
			usageColumnsWithDefault,
			usageColumnsWithoutDefault,
			nzDefaults,
		)
		update := updateColumns.UpdateColumnSet(
			usageAllColumns,
			usagePrimaryKeyColumns,
		)

		if updateOnConflict && len(update) == 0 {
			return errors.New("models: unable to upsert usage, could not build update column list")
		}

		conflict := conflictColumns
		if len(conflict) == 0 {
			conflict = make([]string, len(usagePrimaryKeyColumns))
			copy(conflict, usagePrimaryKeyColumns)
		}
		cache.query = buildUpsertQueryPostgres(dialect, "\"usage\"", updateOnConflict, ret, update, conflict, insert)

		cache.valueMapping, err = queries.BindMapping(usageType, usageMapping, insert)
		if err != nil {
			return err
		}
		if len(ret) != 0 {
			cache.retMapping, err = queries.BindMapping(usageType, usageMapping, ret)
			if err != nil {
				return err
			}
		}
	}

	value := reflect.Indirect(reflect.ValueOf(o))
	vals := queries.ValuesFromMapping(value, cache.valueMapping)
	var returns []interface{}
	if len(cache.retMapping) != 0 {
		returns = queries.PtrsFromMapping(value, cache.retMapping)
	}

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, cache.query)
		fmt.Fprintln(writer, vals)
	}
	if len(cache.retMapping) != 0 {
		err = exec.QueryRowContext(ctx, cache.query, vals...).Scan(returns...)
		if err == sql.ErrNoRows {
			err = nil // Postgres doesn't return anything when there's no update
		}
	} else {
		_, err = exec.ExecContext(ctx, cache.query, vals...)
	}
	if err != nil {
		return errors.Wrap(err, "models: unable to upsert usage")
	}

	if !cached {
		usageUpsertCacheMut.Lock()
		usageUpsertCache[key] = cache
		usageUpsertCacheMut.Unlock()
	}

	return nil
}

// DeleteG deletes a single Usage record.
// DeleteG will match against the primary key column to find the record to delete.
func (o *Usage) DeleteG(ctx context.Context) (int64, error) {
	return o.Delete(ctx, boil.GetContextDB())
}

// Delete deletes a single Usage record with an executor.
// Delete will match against the primary key column to find the record to delete.
func (o *Usage) Delete(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if o == nil {
		return 0, errors.New("models: no Usage provided for delete")
	}

	args := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(o)), usagePrimaryKeyMapping)
	sql := "DELETE FROM \"usage\" WHERE \"id\"=$1"

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args...)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete from usage")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by delete for usage")
	}

	return rowsAff, nil
}

func (q usageQuery) DeleteAllG(ctx context.Context) (int64, error) {
	return q.DeleteAll(ctx, boil.GetContextDB())
}

// DeleteAll deletes all matching rows.
func (q usageQuery) DeleteAll(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if q.Query == nil {
		return 0, errors.New("models: no usageQuery provided for delete all")
	}

	queries.SetDelete(q.Query)

	result, err := q.Query.ExecContext(ctx, exec)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete all from usage")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by deleteall for usage")
	}

	return rowsAff, nil
}

// DeleteAllG deletes all rows in the slice.
func (o UsageSlice) DeleteAllG(ctx context.Context) (int64, error) {
	return o.DeleteAll(ctx, boil.GetContextDB())
}

// DeleteAll deletes all rows in the slice, using an executor.
func (o UsageSlice) DeleteAll(ctx context.Context, exec boil.ContextExecutor) (int64, error) {
	if len(o) == 0 {
		return 0, nil
	}

	var args []interface{}
	for _, obj := range o {
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), usagePrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := "DELETE FROM \"usage\" WHERE " +
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), 1, usagePrimaryKeyColumns, len(o))

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, args)
	}
	result, err := exec.ExecContext(ctx, sql, args...)
	if err != nil {
		return 0, errors.Wrap(err, "models: unable to delete all from usage slice")
	}

	rowsAff, err := result.RowsAffected()
	if err != nil {
		return 0, errors.Wrap(err, "models: failed to get rows affected by deleteall for usage")
	}

	return rowsAff, nil
}

// ReloadG refetches the object from the database using the primary keys.
func (o *Usage) ReloadG(ctx context.Context) error {
	if o == nil {
		return errors.New("models: no Usage provided for reload")
	}

	return o.Reload(ctx, boil.GetContextDB())
}

// Reload refetches the object from the database
// using the primary keys with an executor.
func (o *Usage) Reload(ctx context.Context, exec boil.ContextExecutor) error {
	ret, err := FindUsage(ctx, exec, o.ID)
	if err != nil {
		return err
	}

	*o = *ret
	return nil
}

// ReloadAllG refetches every row with matching primary key column values
// and overwrites the original object slice with the newly updated slice.
func (o *UsageSlice) ReloadAllG(ctx context.Context) error {
	if o == nil {
		return errors.New("models: empty UsageSlice provided for reload all")
	}

	return o.ReloadAll(ctx, boil.GetContextDB())
}

// ReloadAll refetches every row with matching primary key column values
// and overwrites the original object slice with the newly updated slice.
func (o *UsageSlice) ReloadAll(ctx context.Context, exec boil.ContextExecutor) error {
	if o == nil || len(*o) == 0 {
		return nil
	}

	slice := UsageSlice{}
	var args []interface{}
	for _, obj := range *o {
		pkeyArgs := queries.ValuesFromMapping(reflect.Indirect(reflect.ValueOf(obj)), usagePrimaryKeyMapping)
		args = append(args, pkeyArgs...)
	}

	sql := "SELECT \"usage\".* FROM \"usage\" WHERE " +
		strmangle.WhereClauseRepeated(string(dialect.LQ), string(dialect.RQ), 1, usagePrimaryKeyColumns, len(*o))

	q := queries.Raw(sql, args...)

	err := q.Bind(ctx, exec, &slice)
	if err != nil {
		return errors.Wrap(err, "models: unable to reload all in UsageSlice")
	}

	*o = slice

	return nil
}

// UsageExistsG checks if the Usage row exists.
func UsageExistsG(ctx context.Context, iD int64) (bool, error) {
	return UsageExists(ctx, boil.GetContextDB(), iD)
}

// UsageExists checks if the Usage row exists.
func UsageExists(ctx context.Context, exec boil.ContextExecutor, iD int64) (bool, error) {
	var exists bool
	sql := "select exists(select 1 from \"usage\" where \"id\"=$1 limit 1)"

	if boil.IsDebug(ctx) {
		writer := boil.DebugWriterFrom(ctx)
		fmt.Fprintln(writer, sql)
		fmt.Fprintln(writer, iD)
	}
	row := exec.QueryRowContext(ctx, sql, iD)

	err := row.Scan(&exists)
	if err != nil {
		return false, errors.Wrap(err, "models: unable to check if usage exists")
	}

	return exists, nil
}
