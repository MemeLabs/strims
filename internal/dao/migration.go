package dao

import (
	"context"
	"fmt"
	"math"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	networkv1ca "github.com/MemeLabs/strims/pkg/apis/network/v1/ca"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/kv"
	"go.uber.org/zap"
	"google.golang.org/protobuf/reflect/protoreflect"
)

var storeVersion = NewSingleton(
	storeVersionNS,
	&SingletonOptions[daov1.StoreVersion, *daov1.StoreVersion]{
		DefaultValue: &daov1.StoreVersion{
			Version: 0,
		},
	},
)

func checkStoreVersion(s kv.RWStore) (*daov1.StoreVersion, bool, error) {
	v, err := storeVersion.Get(s)
	if err != nil || v.Version == CurrentVersion {
		return v, false, err
	}
	if v.Version > CurrentVersion {
		return v, false, fmt.Errorf("incompatible store version have: %d want: %d", v.Version, CurrentVersion)
	}
	return v, true, nil
}

func Upgrade(ctx context.Context, logger *zap.Logger, s Store) error {
	_, needUpgrade, err := checkStoreVersion(s)
	if !needUpgrade || err != nil {
		return err
	}

	mu := NewMutex(logger, s, "dao", "upgrade")
	ctx, err = mu.Lock(ctx)
	if err != nil {
		return err
	}
	defer mu.Release()

	return s.Update(func(tx kv.RWTx) error {
		v, needUpgrade, err := checkStoreVersion(tx)
		if !needUpgrade || err != nil {
			return err
		}

		logger.Info(
			"store upgrade required",
			zap.Uint32("have", v.Version),
			zap.Uint32("want", CurrentVersion),
		)

		if err := upgrade(s, tx, v.Version); err != nil {
			return err
		}
		if err := storeVersion.Set(tx, &daov1.StoreVersion{Version: CurrentVersion}); err != nil {
			return err
		}

		return ctx.Err()
	})
}

func upgrade(s Store, tx kv.RWTx, v uint32) error {
	switch v {
	case 0:
		if err := upgradeAssignVersion(s, tx, ChatWhisperThreads); err != nil {
			return err
		}
		if err := upgradeAssignVersion(s, tx, ChatWhisperRecords); err != nil {
			return err
		}
		if err := upgradeAssignVersion(s, tx, ChatUIConfigHighlights); err != nil {
			return err
		}
		if err := upgradeAssignVersion(s, tx, ChatUIConfigTags); err != nil {
			return err
		}
		if err := upgradeAssignVersion(s, tx, ChatUIConfigIgnores); err != nil {
			return err
		}
		fallthrough
	case 1:
		if err := UnreadChatWhisperRecordsByPeerKey.rebuild(tx); err != nil {
			return err
		}
		fallthrough
	case 2:
		if err := initProfileDevice(tx); err != nil {
			return err
		}
		fallthrough
	case 3:
		if err := upgradeAssignVersion(s, tx, Networks); err != nil {
			return err
		}
		if err := upgradeAssignVersion(s, tx, BootstrapClients); err != nil {
			return err
		}
		fallthrough
	case 4:
		ProfileID.t.Transform(tx, func(p *profilev1.ProfileID) error {
			p.LastId = math.MaxUint64
			return nil
		})
		fallthrough
	case 5:
		emotes, err := ChatEmotes.GetAll(tx)
		if err != nil {
			return err
		}
		for _, e := range emotes {
			e.Enable = true
			if err := ChatEmotes.Update(tx, e); err != nil {
				return err
			}
		}
		fallthrough
	case 6:
		type logPair struct {
			first, last *networkv1ca.CertificateLog
		}
		peerLogs := hashmap.NewBytesTo[*logPair]()
		ls, err := CertificateLogs.GetAll(tx)
		if err != nil {
			return err
		}
		for _, l := range ls {
			prev, ok := peerLogs.Get(l.Certificate.Key)
			if !ok {
				peerLogs.Set(l.Certificate.Key, &logPair{l, l})
			} else {
				if prev.first.Certificate.NotBefore > l.Certificate.NotBefore {
					prev.first = l
				}
				if prev.last.Certificate.NotAfter < l.Certificate.NotAfter {
					prev.last = l
				}
			}
		}
		profile, err := Profile.Get(tx)
		if err != nil {
			return err
		}
		for it := peerLogs.Iterate(); it.Next(); {
			l := it.Value().last
			p, err := NewNetworkPeer(ProfileID.IDGenerator(tx), l.NetworkId, l.Certificate.Key, l.Certificate.Subject, profile.Id)
			if err != nil {
				return err
			}
			p.CreatedAt = int64(it.Value().first.Certificate.NotBefore)
			if err := NetworkPeers.Insert(tx, p); err != nil {
				return err
			}
			r, err := NewNetworkAliasReservation(ProfileID.IDGenerator(tx), l.NetworkId, l.Certificate.Subject, l.Certificate.Key)
			if err != nil {
				return err
			}
			if err := NetworkAliasReservations.Insert(tx, r); err != nil {
				return err
			}
		}
		fallthrough
	default:
		return nil
	}
}

func upgradeAssignVersion[M any, R ReplicatedTableRecord[M]](s Store, tx kv.RWTx, t *Table[M, R]) error {
	ms, err := t.GetAll(tx)
	if err != nil {
		return err
	}

	d, err := versionvector.ProtoFieldDescriptor[R]()
	if err != nil {
		return fmt.Errorf("upgrade failed: %w", err)
	}

	for _, m := range ms {
		m.ProtoReflect().Set(d, protoreflect.ValueOf(versionvector.NewSeed(s.ReplicaID()).ProtoReflect()))
		if err := t.Update(tx, m); err != nil {
			return err
		}
	}
	return nil
}
