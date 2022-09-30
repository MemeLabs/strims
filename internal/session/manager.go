// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package session

import (
	"context"
	"strconv"

	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/queue"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

// VPNFunc ...
type VPNFunc func(key *key.Key) (*vpn.Host, error)

type Session struct {
	// maybe attached clients? or a count at least?

	Profile *profilev1.Profile
	Store   dao.Store
	Queue   queue.Queue
	App     app.Control
}

func NewManager(
	logger *zap.Logger,
	store kv.BlobStore,
	queue queue.Transport,
	newVPN VPNFunc,
	broker network.Broker,
	httpmux *httputil.MapServeMux,
) *Manager {
	return &Manager{
		logger:   logger,
		store:    store,
		queue:    queue,
		newVPN:   newVPN,
		broker:   broker,
		httpmux:  httpmux,
		sessions: map[uint64]*Session{},
	}
}

type Manager struct {
	logger   *zap.Logger
	store    kv.BlobStore
	queue    queue.Transport
	newVPN   VPNFunc
	broker   network.Broker
	httpmux  *httputil.MapServeMux
	sessions map[uint64]*Session
}

func (t *Manager) GetOrCreateSession(profileID uint64, profileKey []byte) (*Session, error) {
	if session, ok := t.sessions[profileID]; ok {
		return session, nil
	}

	storageKey, err := dao.NewStorageKeyFromBytes(profileKey, nil)
	if err != nil {
		return nil, err
	}

	queue, err := t.queue.Open(strconv.FormatUint(profileID, 10))
	if err != nil {
		return nil, err
	}
	observers := event.NewObservers(t.logger, queue, storageKey)

	store, err := dao.NewReplicatedStore(dao.NewProfileStore(profileID, storageKey, t.store, &dao.ProfileStoreOptions{EventEmitter: dao.EventEmitterFunc(observers.EmitGlobal)}))
	if err != nil {
		return nil, err
	}

	err = dao.Upgrade(context.Background(), t.logger, store)
	if err != nil {
		return nil, err
	}

	profile, err := dao.Profile.Get(store)
	if err != nil {
		return nil, err
	}

	vpn, err := t.newVPN(profile.Key)
	if err != nil {
		return nil, err
	}

	app := app.NewControl(context.Background(), t.logger, vpn, store, observers, t.httpmux, t.broker, profile)

	session := &Session{
		Profile: profile,
		Store:   store,
		Queue:   queue,
		App:     app,
	}
	t.sessions[profileID] = session
	return session, nil
}
