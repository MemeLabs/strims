package rpc

import (
	"context"
	"encoding/base64"
	"errors"
	"strconv"
	"strings"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type sessionContextKeyType struct{}

var sessionContextKey sessionContextKeyType

func contextWithSession(ctx context.Context, s *Session) context.Context {
	return context.WithValue(ctx, sessionContextKey, s)
}

// ContextSession ...
func ContextSession(ctx context.Context) *Session {
	return ctx.Value(sessionContextKey).(*Session)
}

var errMalformedSessionID = errors.New("malformed session id")

// UnmarshalSessionID ...
func UnmarshalSessionID(id string) (uint64, *dao.StorageKey, error) {
	i := strings.IndexRune(id, '.')
	if i == -1 {
		return 0, nil, errMalformedSessionID
	}

	profileID, err := strconv.ParseUint(id[:i], 36, 64)
	if err != nil {
		return 0, nil, err
	}

	kb, err := base64.RawURLEncoding.DecodeString(id[i+1:])
	if err != nil {
		return 0, nil, err
	}
	storageKey := dao.NewStorageKeyFromBytes(kb)

	return profileID, storageKey, nil
}

func newSession() *Session {
	return &Session{}
}

// Session ...
type Session struct {
	lock    sync.Mutex
	profile *pb.Profile
	store   *dao.ProfileStore
	// TODO: private...
	Values sync.Map
}

// Reset ...
func (s *Session) Reset() {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.profile = nil
	s.store = nil
}

// Init ...
func (s *Session) Init(profile *pb.Profile, store *dao.ProfileStore) {
	s.lock.Lock()
	defer s.lock.Unlock()

	s.profile = profile
	s.store = store
}

// Anonymous ...
func (s *Session) Anonymous() bool {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.profile.Id == 0
}

// Store ...
func (s *Session) Store() *dao.ProfileStore {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.store
}

// ProfileID ...
func (s *Session) ProfileID() uint64 {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.profile.Id
}

// Profile ...
func (s *Session) Profile() *pb.Profile {
	s.lock.Lock()
	defer s.lock.Unlock()
	return s.profile
}

// ID ...
// TODO: persist profile keys somewhere else if we want sessions to survive restarts
func (s *Session) ID() string {
	s.lock.Lock()
	defer s.lock.Unlock()

	id := strconv.FormatUint(s.profile.Id, 36)
	storageKey := base64.RawURLEncoding.EncodeToString(s.store.Key().Key())
	return id + "." + storageKey
}
