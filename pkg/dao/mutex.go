package dao

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"log"
	"math"
	"math/rand"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const mutexTTL = 30 * time.Second
const mutexRefreshInterval = 20 * time.Second
const mutexRecheckMinInterval = 20 * time.Second
const mutexRecheckMaxInterval = 40 * time.Second

// NewMutex ...
func NewMutex(store *ProfileStore, key []byte) *Mutex {
	token := make([]byte, 8)
	binary.BigEndian.PutUint64(token, rand.Uint64())

	return &Mutex{
		store: store,
		key:   fmt.Sprintf("mutex:%x", key),
		token: token,
	}
}

// Mutex ...
type Mutex struct {
	store *ProfileStore
	table string
	key   string
	token []byte
}

// Lock ...
func (m *Mutex) Lock(ctx context.Context) error {
	var nextTick time.Duration
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case t := <-time.After(nextTick):
			if err := m.tryLock(t); err != nil {
				log.Println("lock err", err)
				fuzz := mutexRecheckMaxInterval - mutexRecheckMinInterval
				nextTick = mutexRecheckMinInterval + fuzz*time.Duration(rand.Int31())/time.Duration(math.MaxInt32)
			} else {
				log.Println("lock acquired")
				nextTick = mutexRefreshInterval
			}
		}
	}
}

func (m *Mutex) tryLock(t time.Time) error {
	return m.store.Update(func(tx RWTx) error {
		now := t.UnixNano()

		mu := &pb.Mutex{}
		err := tx.Get(m.key, mu)
		if err != nil && err != ErrRecordNotFound {
			return err
		}

		if mu != nil && mu.Eol > now && !bytes.Equal(mu.Token, m.token) {
			return errors.New("held by someone else")
		}

		return tx.Put(m.key, &pb.Mutex{
			Eol:   now + int64(mutexTTL),
			Token: m.token,
		})
	})
}

// Release ...
func (m *Mutex) Release() error {
	return m.store.Update(func(tx RWTx) error {
		mu := &pb.Mutex{}
		err := tx.Get(m.key, mu)
		if err != nil {
			return err
		}

		if !bytes.Equal(mu.Token, m.token) {
			return errors.New("held by someone else")
		}

		return tx.Delete(m.key)
	})
}
