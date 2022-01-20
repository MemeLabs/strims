package session

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/peer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// VPNFunc ...
type VPNFunc func(key *key.Key) (*vpn.Host, error)

type Session struct {
	// maybe attached clients? or a count at least?

	Profile *profilev1.Profile
	Store   *dao.ProfileStore
	App     app.Control
}

func NewManager(logger *zap.Logger, store kv.BlobStore, newVPN VPNFunc, broker network.Broker) *Manager {
	return &Manager{
		logger:   logger,
		store:    store,
		newVPN:   newVPN,
		broker:   broker,
		sessions: map[uint64]*Session{},
	}
}

type Manager struct {
	logger   *zap.Logger
	store    kv.BlobStore
	newVPN   VPNFunc
	broker   network.Broker
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
	store := dao.NewProfileStore(profileID, t.store, storageKey)

	profile, err := dao.Profile.Get(store)
	if err != nil {
		return nil, err
	}

	vpn, err := t.newVPN(profile.Key)
	if err != nil {
		return nil, err
	}

	app := app.NewControl(t.logger, t.broker, vpn, store, profile)
	go app.Run(context.Background())

	qosc := vpn.VNIC().QOS().AddClass(1)
	vpn.VNIC().AddPeerHandler(peer.NewPeerHandler(t.logger, app, store, qosc))

	session := &Session{
		Profile: profile,
		Store:   store,
		App:     app,
	}
	t.sessions[profileID] = session
	return session, nil
}
