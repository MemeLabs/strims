package dao

import (
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
)

const metadataTable = "default"

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
	ErrAuthenticationRequired  = errors.New("method requires authentication")
)

// NewMetadataStore ...
func NewMetadataStore(store Store) (*MetadataStore, error) {
	if err := store.CreateStoreIfNotExists(metadataTable); err != nil {
		return nil, err
	}

	return &MetadataStore{
		store: store,
	}, nil
}

// MetadataStore ...
type MetadataStore struct {
	store Store
	key   *StorageKey
}

// CreaCreateStoreIfNotExists ...
func (s *MetadataStore) CreateStoreIfNotExists(table string) error {
	return fmt.Errorf("unimplemented")
}

// CreateProfile ...
func (s *MetadataStore) CreateProfile(name, password string) (*pb.Profile, *ProfileStore, error) {
	profile, err := NewProfile(name)
	if err != nil {
		return nil, nil, err
	}
	storageKey, err := NewStorageKey(password)
	if err != nil {
		return nil, nil, err
	}

	err = s.store.Update(metadataTable, func(tx Tx) error {
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

	store := NewProfileStore(profile.Id, s.store, storageKey)
	if err := store.Init(profile); err != nil {
		return nil, nil, err
	}

	return profile, store, nil
}

// DeleteProfile ...
func (s *MetadataStore) DeleteProfile(profile *pb.Profile) error {
	return s.store.Update(metadataTable, func(tx Tx) error {
		return tx.Delete(prefixProfileSummaryKey(profile.Name))
	})
}

// LoadProfile ...
func (s *MetadataStore) LoadProfile(id uint64, password string) (*pb.Profile, *ProfileStore, error) {
	profile := &pb.Profile{}
	var storageKey *StorageKey

	err := s.store.View(prefixProfileKey(id), func(tx Tx) (err error) {
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

	store := NewProfileStore(id, s.store, storageKey)
	return profile, store, nil
}

// LoadSession ...
func (s *MetadataStore) LoadSession(id uint64, storageKey *StorageKey) (*pb.Profile, *ProfileStore, error) {
	profile := &pb.Profile{}
	err := s.store.View(prefixProfileKey(id), func(tx Tx) (err error) {
		return get(tx, storageKey, "profile", profile)
	})
	if err != nil {
		return nil, nil, err
	}

	store := NewProfileStore(id, s.store, storageKey)
	return profile, store, nil
}

// GetProfiles ...
func (s *MetadataStore) GetProfiles() ([]*pb.ProfileSummary, error) {
	var profileBufs [][]byte
	err := s.store.View(metadataTable, func(tx Tx) (err error) {
		profileBufs, err = tx.ScanPrefix(profileSummaryKeyPrefix)
		return err
	})
	if err != nil {
		return nil, err
	}

	profiles, err := appendUnmarshalled([]*pb.ProfileSummary{}, profileBufs...)
	if err != nil {
		return nil, err
	}

	return profiles.([]*pb.ProfileSummary), nil
}
