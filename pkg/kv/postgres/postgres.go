package postgres

import (
	"context"
	"database/sql"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	_ "github.com/lib/pq"
)

// NewStore ...
func NewStore(connStr string) (kv.BlobStore, error) {
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, err
	}
	return &Store{db}, nil
}

// Store ...
type Store struct {
	db *sql.DB
}

// Close ...
func (s *Store) Close() error {
	s.db.Close()
	return nil
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	_, err := s.db.Exec(fmt.Sprintf(`CREATE TABLE IF NOT EXISTS "%s" ("key" varchar(128) PRIMARY KEY, "value" bytea)`, table))
	return err
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	_, err := s.db.Query(fmt.Sprintf(`DROP TABLE IF EXISTS "%s"`, table))
	return err
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, fn, true)
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, fn, false)
}

func (s *Store) transact(table string, fn func(tx kv.BlobTx) error, readOnly bool) error {
	tx, err := s.db.BeginTx(context.TODO(), &sql.TxOptions{ReadOnly: readOnly})
	if err != nil {
		return err
	}
	if err := fn(Tx{table, tx}); err != nil {
		if err := tx.Rollback(); err != nil {
			return err
		}
		return err
	}
	return tx.Commit()
}

// Tx ...
type Tx struct {
	table string
	tx    *sql.Tx
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	_, err := t.tx.Exec(fmt.Sprintf(`INSERT INTO "%[1]s" VALUES($1, $2) ON CONFLICT ON CONSTRAINT "%[1]s_pkey" DO UPDATE SET "value" = $2`, t.table), key, value)
	return err
}

// Delete ...
func (t Tx) Delete(key string) error {
	_, err := t.tx.Exec(fmt.Sprintf(`DELETE FROM "%s" WHERE "key" = $1`, t.table), key)
	return err
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	r := t.tx.QueryRow(fmt.Sprintf(`SELECT "value" FROM "%s" WHERE "key" = $1`, t.table), key)
	if err := r.Err(); err != nil {
		return nil, err
	}
	var v []byte
	err = r.Scan(&v)
	if err == sql.ErrNoRows {
		return nil, kv.ErrRecordNotFound
	}
	return v, err
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	rs, err := t.tx.Query(fmt.Sprintf(`SELECT "value" FROM "%s" WHERE "key" LIKE $1`, t.table), prefix+`%`)
	if err != nil {
		return nil, err
	}
	for rs.Next() {
		var v []byte
		if err := rs.Scan(&v); err != nil {
			return nil, err
		}
		values = append(values, v)
	}
	return values, nil
}

func (t Tx) Commit() error {
	return t.tx.Commit()
}
