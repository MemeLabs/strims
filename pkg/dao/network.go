package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const networkPrefix = "network:"

func prefixNetworkKey(id uint64) string {
	return networkPrefix + strconv.FormatUint(id, 10)
}

// InsertNetwork ...
func InsertNetwork(s kv.RWStore, v *pb.Network) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixNetworkKey(v.Id), v)
	})
}

// DeleteNetwork ...
func DeleteNetwork(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Delete(prefixNetworkKey(id))
	})
}

// GetNetwork ...
func GetNetwork(s kv.Store, id uint64) (v *pb.Network, err error) {
	v = &pb.Network{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNetworkKey(id), v)
	})
	return
}

// GetNetworks ...
func GetNetworks(s kv.Store) (v []*pb.Network, err error) {
	v = []*pb.Network{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(networkPrefix, &v)
	})
	return
}

// NewNetwork ...
func NewNetwork(name string) (*pb.Network, error) {
	id, err := GenerateSnowflake()
	if err != nil {
		return nil, err
	}
	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}
	network := &pb.Network{
		Id:   id,
		Name: name,
		Key:  key,
	}
	csr, err := NewCertificateRequest(key, pb.KeyUsage_KEY_USAGE_SIGN, WithSubject(name))
	if err != nil {
		return nil, err
	}

	network.Certificate, err = SignCertificateRequest(csr, defaultCertTTL, key)
	if err != nil {
		return nil, err
	}
	return network, nil
}
