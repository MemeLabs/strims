package dao

import (
	"strconv"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"google.golang.org/protobuf/proto"
)

const profileKeyPrefix = "profile:"

func prefixProfileKey(id uint64) string {
	return profileKeyPrefix + strconv.FormatUint(id, 10)
}

// CreateProfile ...
func CreateProfile(s kv.BlobStore, name, password string) (*profilev1.Profile, *ProfileStore, error) {
	profile, err := NewProfile(name)
	if err != nil {
		return nil, nil, err
	}
	storageKey, err := NewStorageKey(password)
	if err != nil {
		return nil, nil, err
	}

	err = s.Update(metadataTable, func(tx kv.BlobTx) error {
		if ok, err := exists(tx, prefixProfileSummaryKey(name)); err != nil {
			return err
		} else if ok {
			return ErrProfileNameNotAvailable
		}

		b, err := proto.Marshal(NewProfileSummary(profile))
		if err != nil {
			return err
		}
		if err := tx.Put(prefixProfileSummaryKey(name), b); err != nil {
			return err
		}
		return nil
	})
	if err != nil {
		return nil, nil, err
	}

	store := NewProfileStore(profile.Id, s, storageKey)
	if err := store.Init(profile); err != nil {
		return nil, nil, err
	}

	return profile, store, nil
}

// DeleteProfile ...
func DeleteProfile(s kv.BlobStore, profile *profilev1.Profile) error {
	return s.Update(metadataTable, func(tx kv.BlobTx) error {
		return tx.Delete(prefixProfileSummaryKey(profile.Name))
	})
}

// GetProfile ...
func GetProfile(s kv.Store) (v *profilev1.Profile, err error) {
	v = &profilev1.Profile{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get("profile", v)
	})
	return
}

// LoadProfile ...
func LoadProfile(s kv.BlobStore, id uint64, password string) (*profilev1.Profile, *ProfileStore, error) {
	profile := &profilev1.Profile{}
	var storageKey *StorageKey

	err := s.View(prefixProfileKey(id), func(tx kv.BlobTx) (err error) {
		b, err := tx.Get("key")
		if err != nil {
			return err
		}
		storageKey, err = UnmarshalStorageKey(b, password)
		if err != nil {
			return err
		}

		if err := get(tx, storageKey, "profile", profile); err != nil {
			return err
		}
		return nil
	})
	if err != nil {
		return nil, nil, err
	}

	store := NewProfileStore(id, s, storageKey)
	return profile, store, nil
}

// LoadProfileFromSession ...
func LoadProfileFromSession(s kv.BlobStore, id uint64, storageKey *StorageKey) (*profilev1.Profile, *ProfileStore, error) {
	profile := &profilev1.Profile{}
	err := s.View(prefixProfileKey(id), func(tx kv.BlobTx) (err error) {
		return get(tx, storageKey, "profile", profile)
	})
	if err != nil {
		return nil, nil, err
	}

	store := NewProfileStore(id, s, storageKey)
	return profile, store, nil
}

// NewProfile ...
func NewProfile(name string) (p *profilev1.Profile, err error) {
	p = &profilev1.Profile{
		Name: name,
	}

	p.Key, err = GenerateKey()
	if err != nil {
		return nil, err
	}

	p.Id, err = GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	return p, nil
}
