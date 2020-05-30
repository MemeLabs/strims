package dao

import (
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// NewProfileStore ...
func NewProfileStore(profileID uint64, store Store, key *StorageKey) *ProfileStore {
	return &ProfileStore{
		store: store,
		key:   key,
		name:  prefixProfileKey(profileID),
	}
}

// ProfileStore ...
type ProfileStore struct {
	store Store
	key   *StorageKey
	name  string
}

// Init ...
func (s *ProfileStore) Init(profile *pb.Profile) error {
	if err := s.store.CreateStoreIfNotExists(s.name); err != nil {
		return err
	}
	return s.store.Update(s.name, func(tx Tx) error {
		b, err := MarshalStorageKey(s.key)
		if err != nil {
			return err
		}
		if err := tx.Put("key", b); err != nil {
			return err
		}

		if err := put(tx, s.key, "profile", profile); err != nil {
			return err
		}
		return nil
	})
}

// Delete ...
func (s *ProfileStore) Delete() error {
	return s.store.DeleteStore(s.name)
}

// Key ...
func (s *ProfileStore) Key() *StorageKey {
	return s.key
}

// GetProfile ...
func (s *ProfileStore) GetProfile() (*pb.Profile, error) {
	v := &pb.Profile{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return get(tx, s.key, "profile", v)
	})
	if err != nil {
		return nil, err
	}
	return v, nil
}

// InsertNetwork ...
func (s *ProfileStore) InsertNetwork(v *pb.Network) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return put(tx, s.key, prefixNetworkKey(v.Id), v)
	})
}

// DeleteNetwork ...
func (s *ProfileStore) DeleteNetwork(id uint64) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return tx.Delete(prefixNetworkKey(id))
	})
}

// GetNetwork ...
func (s *ProfileStore) GetNetwork(id uint64) (*pb.Network, error) {
	v := &pb.Network{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return get(tx, s.key, prefixNetworkKey(id), v)
	})
	if err != nil {
		return nil, err
	}
	return v, nil
}

// GetNetworks ...
func (s *ProfileStore) GetNetworks() ([]*pb.Network, error) {
	vs := []*pb.Network{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return scanPrefix(tx, s.key, networkPrefix, &vs)
	})
	if err != nil {
		return nil, err
	}
	return vs, nil
}

// InsertNetworkMembership ...
func (s *ProfileStore) InsertNetworkMembership(v *pb.NetworkMembership) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return put(tx, s.key, prefixNetworkMembershipKey(v.Id), v)
	})
}

// DeleteNetworkMembership ...
func (s *ProfileStore) DeleteNetworkMembership(id uint64) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return tx.Delete(prefixNetworkMembershipKey(id))
	})
}

// GetNetworkMembership ...
func (s *ProfileStore) GetNetworkMembership(id uint64) (*pb.NetworkMembership, error) {
	v := &pb.NetworkMembership{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return get(tx, s.key, prefixNetworkMembershipKey(id), v)
	})
	if err != nil {
		return nil, err
	}
	return v, nil
}

// GetNetworkMemberships ...
func (s *ProfileStore) GetNetworkMemberships() ([]*pb.NetworkMembership, error) {
	vs := []*pb.NetworkMembership{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return scanPrefix(tx, s.key, networkMembershipPrefix, &vs)
	})
	if err != nil {
		return nil, err
	}
	return vs, nil
}

// InsertBootstrapClient ...
func (s *ProfileStore) InsertBootstrapClient(v *pb.BootstrapClient) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return put(tx, s.key, prefixBootstrapClientKey(v.Id), v)
	})
}

// DeleteBootstrapClient ...
func (s *ProfileStore) DeleteBootstrapClient(id uint64) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return tx.Delete(prefixBootstrapClientKey(id))
	})
}

// GetBootstrapClient ...
func (s *ProfileStore) GetBootstrapClient(id uint64) (*pb.BootstrapClient, error) {
	v := &pb.BootstrapClient{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return get(tx, s.key, prefixBootstrapClientKey(id), v)
	})
	if err != nil {
		return nil, err
	}
	return v, nil
}

// GetBootstrapClients ...
func (s *ProfileStore) GetBootstrapClients() ([]*pb.BootstrapClient, error) {
	vs := []*pb.BootstrapClient{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return scanPrefix(tx, s.key, bootstrapClientPrefix, &vs)
	})
	if err != nil {
		return nil, err
	}
	return vs, nil
}

// InsertChatServer ...
func (s *ProfileStore) InsertChatServer(v *pb.ChatServer) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return put(tx, s.key, prefixChatServerKey(v.Id), v)
	})
}

// DeleteChatServer ...
func (s *ProfileStore) DeleteChatServer(id uint64) error {
	return s.store.Update(s.name, func(tx Tx) (err error) {
		return tx.Delete(prefixChatServerKey(id))
	})
}

// GetChatServer ...
func (s *ProfileStore) GetChatServer(id uint64) (*pb.ChatServer, error) {
	v := &pb.ChatServer{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return get(tx, s.key, prefixChatServerKey(id), v)
	})
	if err != nil {
		return nil, err
	}
	return v, nil
}

// GetChatServers ...
func (s *ProfileStore) GetChatServers() ([]*pb.ChatServer, error) {
	vs := []*pb.ChatServer{}
	err := s.store.View(s.name, func(tx Tx) (err error) {
		return scanPrefix(tx, s.key, chatServerPrefix, &vs)
	})
	if err != nil {
		return nil, err
	}
	return vs, nil
}
