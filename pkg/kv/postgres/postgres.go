// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package postgres

import (
	"context"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/log/zapadapter"
	"github.com/jackc/pgx/v4/pgxpool"
	"go.uber.org/zap"
)

const transactionTimeout = 10 * time.Second

type Config struct {
	ConnStr       string
	Logger        *zap.Logger
	EnableLogging bool
	MaxConns      int32
}

func NewStoreConfig(cfg Config) (kv.BlobStore, error) {
	pcfg, err := pgxpool.ParseConfig(cfg.ConnStr)
	if err != nil {
		return nil, err
	}
	if cfg.EnableLogging && cfg.Logger != nil {
		pcfg.ConnConfig.Logger = zapadapter.NewLogger(cfg.Logger)
	}
	if cfg.MaxConns != 0 {
		pcfg.MaxConns = cfg.MaxConns
	}

	conn, err := pgxpool.ConnectConfig(context.Background(), pcfg)
	if err != nil {
		return nil, err
	}
	return &Store{conn}, nil
}

func NewStore(connStr string) (kv.BlobStore, error) {
	return NewStoreConfig(Config{ConnStr: connStr})
}

// Store ...
type Store struct {
	db *pgxpool.Pool
}

// Close ...
func (s *Store) Close() error {
	s.db.Close()
	return nil
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	_, err := s.db.Exec(context.Background(), fmt.Sprintf(`CREATE TABLE IF NOT EXISTS "%s" ("key" varchar(128) PRIMARY KEY, "value" bytea)`, table))
	return err
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	_, err := s.db.Query(context.Background(), fmt.Sprintf(`DROP TABLE IF EXISTS "%s"`, table))
	return err
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, fn, pgx.TxOptions{AccessMode: pgx.ReadOnly})
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, fn, pgx.TxOptions{
		AccessMode: pgx.ReadWrite,
	})
}

func (s *Store) transact(table string, fn func(tx kv.BlobTx) error, opt pgx.TxOptions) error {
	ctx, cancel := context.WithTimeout(context.Background(), transactionTimeout)
	defer cancel()

	tx, err := s.db.BeginTx(ctx, opt)
	if err != nil {
		return err
	}
	if err := fn(Tx{ctx, table, tx}); err != nil {
		if err := tx.Rollback(ctx); err != nil {
			return err
		}
		return err
	}
	return tx.Commit(ctx)
}

// Tx ...
type Tx struct {
	ctx   context.Context
	table string
	tx    pgx.Tx
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	_, err := t.tx.Exec(t.ctx, fmt.Sprintf(`INSERT INTO "%[1]s" VALUES(%s, $1) ON CONFLICT ON CONSTRAINT "%[1]s_pkey" DO UPDATE SET "value" = $1`, t.table, escapeString(key)), value)
	return err
}

// Delete ...
func (t Tx) Delete(key string) error {
	_, err := t.tx.Exec(t.ctx, fmt.Sprintf(`DELETE FROM "%s" WHERE "key" = %s`, t.table, escapeString(key)))
	return err
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	r := t.tx.QueryRow(t.ctx, fmt.Sprintf(`SELECT "value" FROM "%s" WHERE "key" = %s`, t.table, escapeString(key)))
	err = r.Scan(&value)
	if err == pgx.ErrNoRows {
		return nil, kv.ErrRecordNotFound
	}
	return value, err
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	return t.ScanCursor(kv.Cursor{Prefix: prefix})
}

// ScanCursor ...
func (t Tx) ScanCursor(cursor kv.Cursor) (values [][]byte, err error) {
	order := "ASC"
	limit := "ALL"
	if cursor.Last != 0 {
		order = "DESC"
		limit = strconv.Itoa(cursor.Last)
	} else if cursor.First != 0 {
		limit = strconv.Itoa(cursor.First)
	}

	var predicates []string
	if cursor.Prefix != "" {
		predicates = append(predicates, `"key" LIKE `+escapeString(cursor.Prefix+"%"))
	}
	if cursor.After != "" {
		predicates = append(predicates, `"key" > `+escapeString(cursor.After))
	}
	if cursor.Before != "" {
		predicates = append(predicates, `"key" < `+escapeString(cursor.Before))
	}
	var whereClause string
	if len(predicates) > 0 {
		whereClause = `WHERE ` + strings.Join(predicates, " AND ")
	}

	rs, err := t.tx.Query(t.ctx, fmt.Sprintf(`SELECT "value" FROM "%s" %s ORDER BY "key" %s LIMIT %s`, t.table, whereClause, order, limit))
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

const lowerhex = "0123456789abcdef"

func escapeString(s string) string {
	buf := make([]byte, 0, 3*len(s)/2)
	buf = append(buf, `'`...)
	for _, r := range s {
		switch r {
		case '\b':
			buf = append(buf, `\b`...)
		case '\f':
			buf = append(buf, `\f`...)
		case '\n':
			buf = append(buf, `\n`...)
		case '\r':
			buf = append(buf, `\r`...)
		case '\t':
			buf = append(buf, `\t`...)
		case '\'':
			buf = append(buf, `''`...)
		default:
			if r > 0x1f && r < 0x7f {
				buf = append(buf, byte(r))
			} else if r < 0x10000 {
				buf = append(buf, `\u`...)
				for s := 12; s >= 0; s -= 4 {
					buf = append(buf, lowerhex[r>>uint(s)&0xF])
				}
			} else {
				buf = append(buf, `\U`...)
				for s := 28; s >= 0; s -= 4 {
					buf = append(buf, lowerhex[r>>uint(s)&0xF])
				}
			}
		}
	}
	buf = append(buf, `'`...)
	return string(buf)
}
