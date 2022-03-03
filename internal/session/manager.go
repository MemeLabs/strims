package session

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/peer"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/httputil"
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

func NewManager(
	logger *zap.Logger,
	store kv.BlobStore,
	newVPN VPNFunc,
	broker network.Broker,
	httpmux *httputil.MapServeMux,
) *Manager {
	return &Manager{
		logger:   logger,
		store:    store,
		newVPN:   newVPN,
		broker:   broker,
		httpmux:  httpmux,
		sessions: map[uint64]*Session{},
	}
}

type Manager struct {
	logger   *zap.Logger
	store    kv.BlobStore
	newVPN   VPNFunc
	broker   network.Broker
	httpmux  *httputil.MapServeMux
	sessions map[uint64]*Session
}

func (t *Manager) GetOrCreateSession(profileID uint64, profileKey []byte) (*Session, error) {
	if session, ok := t.sessions[profileID]; ok {
		return session, nil
	}

	observers := &event.Observers{}

	storageKey, err := dao.NewStorageKeyFromBytes(profileKey, nil)
	if err != nil {
		return nil, err
	}
	store := dao.NewProfileStore(profileID, storageKey, t.store, &dao.ProfileStoreOptions{EventEmitter: observers})

	profile, err := dao.Profile.Get(store)
	if err != nil {
		return nil, err
	}

	vpn, err := t.newVPN(profile.Key)
	if err != nil {
		return nil, err
	}

	app := app.NewControl(context.Background(), t.logger, vpn, store, observers, t.httpmux, t.broker, profile)
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
