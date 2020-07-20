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
	logger *zap.Logger
	store  *ProfileStore
	key    string
	token  []byte
}

// Lock ...
func (m *Mutex) Lock(ctx context.Context) error {
	ch := make(chan error)

	go m.notifyLock(ctx, ch)

	return <-ch
}

func (m *Mutex) notifyLock(ctx context.Context, ch chan error) {
	var held bool
	var nextTick time.Duration

	for {
		select {
		case <-ctx.Done():
			if held {
				if err := m.Release(); err != nil {
					m.logger.Debug("releasing mutex failed", zap.Error(err))
				}
			} else {
				ch <- ctx.Err()
			}
			return

		case t := <-time.After(nextTick):
			if err := m.tryLock(t); err != nil {
				fuzz := mutexRecheckMaxInterval - mutexRecheckMinInterval
				nextTick = mutexRecheckMinInterval + fuzz*time.Duration(rand.Int31())/time.Duration(math.MaxInt32)
				continue
			}

			if !held {
				held = true
				ch <- nil
			}

			nextTick = mutexRefreshInterval
		}
	}
}

func (m *Mutex) tryLock(t time.Time) error {
	return m.store.Update(func(tx kv.RWTx) error {
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
