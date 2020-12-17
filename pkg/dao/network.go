package dao

import (
	"bytes"
	"strconv"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const networkPrefix = "network:"

func prefixNetworkKey(id uint64) string {
	return networkPrefix + strconv.FormatUint(id, 10)
}

// UpsertNetwork ...
func UpsertNetwork(s kv.RWStore, v *pb.Network) error {
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

// GetNetworkByKey ...
func GetNetworkByKey(s kv.Store, key []byte) (*pb.Network, error) {
	networks, err := GetNetworks(s)
	if err != nil {
		return nil, err
	}
	for _, v := range networks {
		if bytes.Equal(GetRootCert(v.Certificate).Key, key) {
			return v, nil
		}
	}
	return nil, kv.ErrRecordNotFound
}

// GetNetworks ...
func GetNetworks(s kv.Store) (v []*pb.Network, err error) {
	v = []*pb.Network{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(networkPrefix, &v)
	})
	return
}

// NewNetworkCertificate ...
func NewNetworkCertificate(network *pb.Network) (*pb.Certificate, error) {
	return NewSelfSignedCertificate(network.Key, pb.KeyUsage_KEY_USAGE_SIGN, defaultCertTTL, WithSubject(network.Name))
}

// NewNetwork ...
func NewNetwork(g IDGenerator, name string, icon *pb.NetworkIcon, profile *pb.Profile) (*pb.Network, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	networkCert, err := NewSelfSignedCertificate(key, pb.KeyUsage_KEY_USAGE_SIGN, defaultCertTTL, WithSubject(name))
	if err != nil {
		return nil, err
	}

	csr, err := NewCertificateRequest(
		profile.Key,
		pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN,
		WithSubject(profile.Name),
	)
	if err != nil {
		return nil, err
	}
	cert, err := SignCertificateRequest(csr, defaultCertTTL, key)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: networkCert}

	return &pb.Network{
		Id:          id,
		Name:        name,
		Key:         key,
		Icon:        icon,
		Certificate: cert,
	}, nil
}

// NewNetworkFromInvitationV0 generates a network from a network invitation
func NewNetworkFromInvitationV0(g IDGenerator, invitation *pb.InvitationV0, profile *pb.Profile) (*pb.Network, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	if err = VerifyCertificate(invitation.Certificate); err != nil {
		return nil, err
	}
	csr, err := NewCertificateRequest(profile.Key, pb.KeyUsage_KEY_USAGE_PEER, WithSubject(profile.Name))
	if err != nil {
		return nil, err
	}
	peerCert, err := SignCertificateRequest(csr, defaultCertTTL, invitation.Key)
	if err != nil {
		return nil, err
	}
	peerCert.ParentOneof = &pb.Certificate_Parent{Parent: invitation.Certificate}

	return &pb.Network{
		Id:          id,
		Name:        GetRootCert(invitation.Certificate).Subject,
		Certificate: peerCert,
	}, nil
}

// NewNetworkFromCertificate generates a network from a network invitation
func NewNetworkFromCertificate(g IDGenerator, cert *pb.Certificate) (*pb.Network, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	if err = VerifyCertificate(cert); err != nil {
		return nil, err
	}

	return &pb.Network{
		Id:          id,
		Name:        GetRootCert(cert).Subject,
		Certificate: cert,
	}, nil
}

// NewInvitationV0 ...
func NewInvitationV0(key *pb.Key, cert *pb.Certificate) (*pb.InvitationV0, error) {
	inviteKey, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	validDuration := time.Hour * 24 * 7 // seven days

	inviteCSR, err := NewCertificateRequest(inviteKey, pb.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}
	inviteCert, err := SignCertificateRequest(inviteCSR, validDuration, key)
	if err != nil {
		return nil, err
	}
	inviteCert.ParentOneof = &pb.Certificate_Parent{Parent: cert}

	return &pb.InvitationV0{
		Key:         inviteKey,
		Certificate: inviteCert,
	}, nil
}
