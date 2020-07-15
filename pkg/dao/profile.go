package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
)

const profileKeyPrefix = "profile:"

func prefixProfileKey(id uint64) string {
	return profileKeyPrefix + strconv.FormatUint(id, 10)
}

// CreateProfile ...
func CreateProfile(s kv.BlobStore, name, password string) (*pb.Profile, *ProfileStore, error) {
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
func DeleteProfile(s kv.BlobStore, profile *pb.Profile) error {
	return s.Update(metadataTable, func(tx kv.BlobTx) error {
		return tx.Delete(prefixProfileSummaryKey(profile.Name))
	})
}

// GetProfile ...
func GetProfile(s kv.Store) (v *pb.Profile, err error) {
	v = &pb.Profile{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get("profile", v)
	})
	return
}

// LoadProfile ...
func LoadProfile(s kv.BlobStore, id uint64, password string) (*pb.Profile, *ProfileStore, error) {
	profile := &pb.Profile{}
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
func LoadProfileFromSession(s kv.BlobStore, id uint64, storageKey *StorageKey) (*pb.Profile, *ProfileStore, error) {
	profile := &pb.Profile{}
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
func NewProfile(name string) (p *pb.Profile, err error) {
	p = &pb.Profile{
		Name: name,
	}

	p.Key, err = GenerateKey()
	if err != nil {
		return nil, err
	}

	p.Id, err = generateSnowflake()
	if err != nil {
		return nil, err
	}

	return p, nil
}
