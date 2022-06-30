// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package postgres

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"time"

	"github.com/MemeLabs/strims/pkg/queue"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/avast/retry-go/v4"
	"github.com/jackc/pgx/v4/log/zapadapter"
	"github.com/jackc/pgx/v4/pgxpool"
	"go.uber.org/zap"
)

const postgresGCIntervalMins = 10
const transactionTimeout = 10 * time.Second

type Config struct {
	ConnStr       string
	Logger        *zap.Logger
	EnableLogging bool
}

func NewTransport(cfg Config) (queue.Transport, error) {
	pcfg, err := pgxpool.ParseConfig(cfg.ConnStr)
	if err != nil {
		return nil, err
	}
	if cfg.EnableLogging && cfg.Logger != nil {
		pcfg.ConnConfig.Logger = zapadapter.NewLogger(cfg.Logger)
	}

	conn, err := pgxpool.ConnectConfig(context.Background(), pcfg)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())
	return &Transport{cfg.Logger, conn, ctx, cancel}, nil
}

// Transport ...
type Transport struct {
	logger *zap.Logger
	db     *pgxpool.Pool
	ctx    context.Context
	cancel context.CancelFunc
}

func (s *Transport) Open(name string) (queue.Queue, error) {
	return newPostgresQueue(s.logger, s.ctx, s.db, name)
}

// Close ...
func (s *Transport) Close() error {
	s.cancel()
	s.db.Close()
	return nil
}

func newPostgresQueue(logger *zap.Logger, ctx context.Context, db *pgxpool.Pool, name string) (*postgresQueue, error) {
	name = fmt.Sprintf("queue_%s", name)
	sql := fmt.Sprintf(``+
		`CREATE TABLE IF NOT EXISTS "%s" (`+
		`  id BIGSERIAL PRIMARY KEY,`+
		`  ts TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,`+
		`  message BYTEA`+
		`);`+
		`CREATE INDEX IF NOT EXISTS "%[1]s_ts_idx" ON "%[1]s" ("ts");`,
		name,
	)
	_, err := db.Exec(ctx, sql)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(ctx)
	q := &postgresQueue{
		logger: logger.With(zap.String("queue", name)),
		name:   name,
		db:     db,
		ctx:    ctx,
		cancel: cancel,
		ids:    memory.NewQueue[int64](),
	}

	go q.run()
	timeutil.DefaultTickEmitter.SubscribeCtx(ctx, postgresGCIntervalMins*time.Minute, q.gc, nil)

	return q, nil
}

type postgresQueue struct {
	logger *zap.Logger
	name   string
	db     *pgxpool.Pool
	ctx    context.Context
	cancel context.CancelFunc
	ids    *memory.Queue[int64]
}

func (q *postgresQueue) gc(t timeutil.Time) {
	_, err := q.db.Exec(q.ctx, fmt.Sprintf(`DELETE FROM "%s" WHERE ts < CURRENT_TIMESTAMP - INTERVAL '%d minute';`, q.name, postgresGCIntervalMins))
	if err != nil {
		q.logger.Warn("gc failed", zap.Error(err))
	}
}

func (q *postgresQueue) run() {
	defer q.Close()

	err := retry.Do(
		q.doRead,
		retry.Attempts(0),
		retry.Context(q.ctx),
		retry.DelayType(func(n uint, err error, config *retry.Config) time.Duration {
			if !errors.Is(err, context.Canceled) {
				q.logger.Warn("reader failed", zap.Error(err))
			}
			return retry.BackOffDelay(n, err, config)
		}),
	)
	q.logger.Debug("reader closed", zap.Error(err))
}

func (q *postgresQueue) doRead() error {
	q.logger.Debug("starting reader")
	c, err := q.db.Acquire(q.ctx)
	if err != nil {
		return err
	}
	defer c.Release()

	_, err = c.Exec(q.ctx, fmt.Sprintf(`LISTEN "%s";`, q.name))
	if err != nil {
		return err
	}

	for {
		notif, err := c.Conn().WaitForNotification(q.ctx)
		if err != nil {
			return err
		}

		id, err := strconv.ParseInt(notif.Payload, 10, 64)
		if err != nil {
			return err
		}

		if err := q.ids.Write(id); err != nil {
			return err
		}
	}
}

func (q *postgresQueue) Read() (any, error) {
	id, err := q.ids.Read()
	if err != nil {
		return nil, err
	}

	r := q.db.QueryRow(q.ctx, fmt.Sprintf(`SELECT "message" FROM "%s" WHERE id = $1;`, q.name), id)
	if err != nil {
		return nil, err
	}
	var v []byte
	if err := r.Scan(&v); err != nil {
		return nil, err
	}
	return v, nil
}

func (q *postgresQueue) Write(m any) error {
	b, ok := m.([]byte)
	if !ok {
		return errors.New("incompatible message type")
	}

	sql := fmt.Sprintf(``+
		`WITH t AS (`+
		`  INSERT INTO "%s" (message) VALUES ($1) RETURNING id`+
		`)`+
		`SELECT pg_notify($2, id::TEXT) FROM t`,
		q.name,
	)
	_, err := q.db.Exec(q.ctx, sql, b, q.name)
	return err
}

func (q *postgresQueue) Close() error {
	q.cancel()
	q.ids.Close()
	return nil
}

func (q *postgresQueue) Cleanup() error {
	_, err := q.db.Exec(context.Background(), fmt.Sprintf(`DROP TABLE IF EXISTS "%s"`, q.name))
	return err
}
