package dao

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"strconv"
	"time"

	authv1 "github.com/MemeLabs/go-ppspp/pkg/apis/auth/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"google.golang.org/protobuf/proto"
)

func CreateServerAuthThing(s kv.BlobStore, name, password string) (uint64, []byte, error) {
	profile, err := NewProfile(name)
	if err != nil {
		return 0, nil, err
	}

	key, err := NewStorageKey(password)
	if err != nil {
		return 0, nil, err
	}

	profileKey := make([]byte, KeySize)
	if _, err = rand.Read(profileKey); err != nil {
		return 0, nil, err
	}

	secret, err := proto.Marshal(&authv1.ServerUserThing_Password_Secret{
		ProfileId:  profile.Id,
		ProfileKey: profileKey,
	})
	if err != nil {
		return 0, nil, err
	}

	secret, err = key.Seal(secret)
	if err != nil {
		return 0, nil, err
	}

	userb, err := proto.Marshal(&authv1.ServerUserThing{
		Name: name,
		Credentials: &authv1.ServerUserThing_Password_{
			Password: &authv1.ServerUserThing_Password{
				AuthKey: key.record,
				Secret:  secret,
			},
		},
	})
	if err != nil {
		return 0, nil, err
	}

	if err = s.CreateStoreIfNotExists("profiles"); err != nil {
		return 0, nil, err
	}
	if err = s.CreateStoreIfNotExists("sessions"); err != nil {
		return 0, nil, err
	}

	err = s.Update("profiles", func(tx kv.BlobTx) error {
		_, err := tx.Get(name)
		if err != kv.ErrRecordNotFound {
			if err != nil {
				return err
			}
			return errors.New("username already taken")
		}
		return tx.Put(name, userb)
	})
	if err != nil {
		return 0, nil, err
	}

	profileStorageKey, _ := NewStorageKeyFromBytes(profileKey, nil)
	store := NewProfileStore(profile.Id, profileStorageKey, s, nil)
	if err = store.Init(); err != nil {
		return 0, nil, err
	}
	err = store.Update(func(tx kv.RWTx) error {
		return Profile.Set(tx, profile)
	})
	if err != nil {
		return 0, nil, err
	}

	return profile.Id, profileKey, nil
}

func LoadServerAuthThing(s kv.BlobStore, name, password string) (uint64, []byte, error) {
	user := &authv1.ServerUserThing{}
	err := s.View("profiles", func(tx kv.BlobTx) error {
		b, err := tx.Get(name)
		if err != nil {
			return err
		}
		return proto.Unmarshal(b, user)
	})
	if err != nil {
		return 0, nil, err
	}

	switch c := user.Credentials.(type) {
	case *authv1.ServerUserThing_Password_:
		key, err := NewStorageKeyFromPassword(password, c.Password.AuthKey)
		if err != nil {
			return 0, nil, err
		}
		secretb, err := key.Open(c.Password.Secret)
		if err != nil {
			return 0, nil, err
		}
		secret := &authv1.ServerUserThing_Password_Secret{}
		if err := proto.Unmarshal(secretb, secret); err != nil {
			return 0, nil, err
		}
		return secret.ProfileId, secret.ProfileKey, nil
	case *authv1.ServerUserThing_Unencrypted_:
		return c.Unencrypted.ProfileId, c.Unencrypted.ProfileKey, nil
	default:
		return 0, nil, errors.New("unsupported credentials type")
	}
}

func NewSessionToken() (*SessionToken, error) {
	t := &SessionToken{
		EOL:   uint64(timeutil.Now().Add(30 * 24 * time.Hour).Unix()),
		Token: make([]byte, 32),
	}
	if _, err := rand.Read(t.Token); err != nil {
		return nil, err
	}
	return t, nil
}

type SessionToken struct {
	EOL   uint64
	Token []byte
}

func (t *SessionToken) String() string {
	return strconv.FormatUint(t.EOL, 10) + ":" + base64.URLEncoding.EncodeToString(t.Token)
}

func (t *SessionToken) Binary() []byte {
	b := make([]byte, 40)
	binary.BigEndian.PutUint64(b, t.EOL)
	copy(b[8:], t.Token)
	return b
}

func UnmarshalSessionToken(b []byte) (*SessionToken, error) {
	if len(b) != 40 {
		return nil, errors.New("incorrect token length")
	}
	return &SessionToken{
		EOL:   binary.BigEndian.Uint64(b),
		Token: b[8:],
	}, nil
}

func CreateSessionThing(s kv.BlobStore, sessionKey []byte, profileID uint64, profileKey []byte) (*SessionToken, error) {
	key, _ := NewStorageKeyFromBytes(sessionKey, nil)

	sessionToken, err := NewSessionToken()
	if err != nil {
		return nil, err
	}

	b, err := proto.Marshal(&authv1.SessionThing{
		ProfileId:  profileID,
		ProfileKey: profileKey,
	})
	b, err = key.Seal(b)
	if err != nil {
		return nil, err
	}

	err = s.Update("sessions", func(tx kv.BlobTx) error {
		return tx.Put(sessionToken.String(), b)
	})
	if err != nil {
		return nil, err
	}

	return sessionToken, nil
}

func LoadSessionThing(s kv.BlobStore, sessionKey []byte, sessionToken *SessionToken) (uint64, []byte, error) {
	key, _ := NewStorageKeyFromBytes(sessionKey, nil)

	var b []byte
	err := s.View("sessions", func(tx kv.BlobTx) (err error) {
		b, err = tx.Get(sessionToken.String())
		return
	})
	if err != nil {
		return 0, nil, err
	}

	b, err = key.Open(b)
	if err != nil {
		return 0, nil, err
	}
	session := &authv1.SessionThing{}
	if err := proto.Unmarshal(b, session); err != nil {
		return 0, nil, err
	}

	return session.ProfileId, session.ProfileKey, nil
}
