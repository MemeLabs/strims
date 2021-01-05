package dao

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"math"
	"math/rand"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

// ErrLockBusy ...
var ErrLockBusy = errors.New("failed to update busy lock")

const mutexTTL = 30 * time.Second
const mutexRefreshInterval = 20 * time.Second
const mutexRecheckMinInterval = 20 * time.Second
const mutexRecheckMaxInterval = 40 * time.Second

// NewMutex ...
func NewMutex(logger *zap.Logger, store *ProfileStore, key []byte) *Mutex {
	token := make([]byte, 8)
	binary.BigEndian.PutUint64(token, rand.Uint64())

	return &Mutex{
		logger: logger,
		store:  store,
		key:    fmt.Sprintf("mutex:%x", key),
		token:  token,
	}
}

// Mutex ...
type Mutex struct {
	logger   *zap.Logger
	store    *ProfileStore
	key      string
	token    []byte
	held     bool
	nextTick time.Duration
}

// Lock ...
func (m *Mutex) Lock(ctx context.Context) (context.Context, error) {
	ch := make(chan error)
	ctx, cancel := context.WithCancel(ctx)
	go m.poll(ctx, cancel, ch)

	return ctx, <-ch
}

// TryLock ...
func (m *Mutex) TryLock(ctx context.Context) (context.Context, error) {
	if err := m.try(time.Now()); err != nil {
		return nil, err
	}

	m.held = true

	ctx, cancel := context.WithCancel(ctx)
	go m.poll(ctx, cancel, nil)

	return ctx, nil
}

func (m *Mutex) poll(ctx context.Context, cancel context.CancelFunc, ch chan error) {
	defer cancel()

	for {
		select {
		case <-ctx.Done():
			if m.held {
				if err := m.Release(); err != nil {
					m.logger.Debug("releasing mutex failed", zap.Error(err))
				}
			} else {
				ch <- ctx.Err()
			}
			return

		case t := <-time.After(m.nextTick):
			if err := m.try(t); err == nil && !m.held {
				m.held = true
				ch <- nil
			} else if err != nil && m.held {
				m.held = false
				return
			}
		}
	}
}

func (m *Mutex) try(t time.Time) error {
	err := m.store.Update(func(tx kv.RWTx) error {
		now := t.UnixNano()

		mu := &pb.Mutex{}
		err := tx.Get(m.key, mu)
		if err != nil && err != kv.ErrRecordNotFound {
			return err
		}

		if mu != nil && mu.Eol > now && !bytes.Equal(mu.Token, m.token) {
			return ErrLockBusy
		}

		return tx.Put(m.key, &pb.Mutex{
			Eol:   now + int64(mutexTTL),
			Token: m.token,
		})
	})

	if err != nil {
		fuzz := mutexRecheckMaxInterval - mutexRecheckMinInterval
		m.nextTick = mutexRecheckMinInterval + fuzz*time.Duration(rand.Int31())/time.Duration(math.MaxInt32)
		return err
	}

	m.nextTick = mutexRefreshInterval
	return nil
}

// Release ...
func (m *Mutex) Release() error {
	return m.store.Update(func(tx kv.RWTx) error {
		mu := &pb.Mutex{}
		err := tx.Get(m.key, mu)
		if err != nil {
			return err
		}

		if !bytes.Equal(mu.Token, m.token) {
			return ErrLockBusy
		}

		return tx.Delete(m.key)
	})
}
