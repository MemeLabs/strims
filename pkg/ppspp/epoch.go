package ppspp

import (
	"errors"
	"sync"

	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

var errEpochOutOfDate = errors.New("epoch is out of date")

func newEpoch(v integrity.SignatureVerifier) epoch {
	return epoch{
		verifier:  v,
		Timestamp: timeutil.NilTime,
		Signature: make([]byte, v.Size()),
	}
}

type epoch struct {
	verifier integrity.SignatureVerifier

	mu        sync.Mutex
	Timestamp timeutil.Time
	Signature []byte
}

func (e *epoch) ImportCache(c *swarmpb.Cache_Epoch) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	e.Timestamp = timeutil.New(c.GetTimestamp())

	e.Signature = make([]byte, e.verifier.Size())
	copy(e.Signature, c.GetSignature())

	return nil
}

func (e *epoch) ExportCache() *swarmpb.Cache_Epoch {
	e.mu.Lock()
	defer e.mu.Unlock()

	return &swarmpb.Cache_Epoch{
		Timestamp: e.Timestamp.UnixNano(),
		Signature: e.Signature,
	}
}

func (e *epoch) Value() (timeutil.Time, []byte) {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.Timestamp, e.Signature
}

func (e *epoch) Sync(t timeutil.Time, sig []byte) (bool, error) {
	e.mu.Lock()
	defer e.mu.Unlock()

	if t == e.Timestamp {
		return false, nil
	}
	if t < e.Timestamp {
		return false, errEpochOutOfDate
	}
	if !e.verifier.Verify(t, nil, sig) {
		return false, integrity.ErrInvalidSignature
	}

	e.Timestamp = t
	e.Signature = append(([]byte)(nil), sig...)

	return true, nil
}
