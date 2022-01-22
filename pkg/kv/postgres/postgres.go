package postgres

import (
	"context"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/log/zapadapter"
	"github.com/jackc/pgx/v4/pgxpool"
	"go.uber.org/zap"
)

type Config struct {
	ConnStr string
	Logger  *zap.Logger
}

// NewStore ...
func NewStoreConfig(cfg *Config) (kv.BlobStore, error) {
	pcfg, err := pgxpool.ParseConfig(cfg.ConnStr)
	if err != nil {
		return nil, err
	}
	if cfg.Logger != nil {
		pcfg.ConnConfig.Logger = zapadapter.NewLogger(cfg.Logger)
	}

	conn, err := pgxpool.ConnectConfig(context.Background(), pcfg)
	if err != nil {
		return nil, err
	}
	return &Store{conn}, nil
}

func NewStore(connStr string) (kv.BlobStore, error) {
	return NewStoreConfig(&Config{ConnStr: connStr})
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
		IsoLevel:   pgx.Serializable,
		AccessMode: pgx.ReadWrite,
	})
}

func (s *Store) transact(table string, fn func(tx kv.BlobTx) error, opt pgx.TxOptions) error {
	ctx := context.Background()
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
	_, err := t.tx.Exec(t.ctx, fmt.Sprintf(`INSERT INTO "%[1]s" VALUES($1, $2) ON CONFLICT ON CONSTRAINT "%[1]s_pkey" DO UPDATE SET "value" = $2`, t.table), key, value)
	return err
}

// Delete ...
func (t Tx) Delete(key string) error {
	_, err := t.tx.Exec(t.ctx, fmt.Sprintf(`DELETE FROM "%s" WHERE "key" = $1`, t.table), key)
	return err
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	r := t.tx.QueryRow(t.ctx, fmt.Sprintf(`SELECT "value" FROM "%s" WHERE "key" = $1`, t.table), key)
	err = r.Scan(&value)
	if err == pgx.ErrNoRows {
		return nil, kv.ErrRecordNotFound
	}
	return value, err
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	rs, err := t.tx.Query(t.ctx, fmt.Sprintf(`SELECT "value" FROM "%s" WHERE "key" LIKE $1`, t.table), prefix+`%`)
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
