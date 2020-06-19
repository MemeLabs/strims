package dao

import (
	"bytes"
	"fmt"
	"strconv"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const networkMembershipPrefix = "networkMembership:"

func prefixNetworkMembershipKey(id uint64) string {
	return networkMembershipPrefix + strconv.FormatUint(id, 10)
}

// InsertNetworkMembership ...
func InsertNetworkMembership(s RWStore, v *pb.NetworkMembership) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Put(prefixNetworkMembershipKey(v.Id), v)
	})
}

// DeleteNetworkMembership ...
func DeleteNetworkMembership(s RWStore, id uint64) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Delete(prefixNetworkMembershipKey(id))
	})
}

// GetNetworkMembership ...
func GetNetworkMembership(s Store, id uint64) (v *pb.NetworkMembership, err error) {
	v = &pb.NetworkMembership{}
	err = s.View(func(tx Tx) error {
		return tx.Get(prefixNetworkMembershipKey(id), v)
	})
	return
}

// GetNetworkMemberships ...
func GetNetworkMemberships(s Store) (v []*pb.NetworkMembership, err error) {
	v = []*pb.NetworkMembership{}
	err = s.View(func(tx Tx) error {
		return tx.ScanPrefix(networkMembershipPrefix, &v)
	})
	return
}

// GetNetworkMembershipByNetworkKey returns the membership belonging to the given network
func GetNetworkMembershipByNetworkKey(s Store, k []byte) (*pb.NetworkMembership, error) {
	memberships, err := GetNetworkMemberships(s)
	if err != nil {
		return nil, err
	}

	for _, im := range memberships {
		if bytes.Equal(GetRootCert(im.Certificate).Key, k) {
			return im, nil
		}
	}
	return nil, ErrRecordNotFound
}

// GetNetworkMembershipForNetwork returns the membership belonging to the given network
func GetNetworkMembershipForNetwork(s Store, netID uint64) (*pb.NetworkMembership, error) {
	network, err := GetNetwork(s, netID)
	if err != nil {
		if err == ErrRecordNotFound {
			return nil, fmt.Errorf("could not find network: %w", err)
		}
		return nil, err
	}

	return GetNetworkMembershipByNetworkKey(s, network.Certificate.Key)
}

// NewNetworkMembershipFromNetwork generates a network membership from a network
func NewNetworkMembershipFromNetwork(network *pb.Network, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	return NewNetworkMembershipFromCSR(network.Name, network.Certificate, network.Certificate, network.Key, csr)
}

// NewNetworkMembershipFromInvite generates a network membership from a network invitation
func NewNetworkMembershipFromInvite(invite *pb.InvitationV0, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	networkCert := GetRootCert(invite.Certificate)
	return NewNetworkMembershipFromCSR(invite.NetworkName, networkCert, invite.Certificate, invite.Key, csr)
}

// NewNetworkMembershipFromCSR generates a new network membership using the given parameters
func NewNetworkMembershipFromCSR(networkName string, networkCert *pb.Certificate, parentCert *pb.Certificate, csrSigningKey *pb.Key, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	cert, err := SignCertificateRequest(csr, defaultCertTTL, csrSigningKey)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: parentCert}

	return NewNetworkMembershipFromCertificate(networkName, cert)
}

// NewNetworkMembershipFromCertificate ...
func NewNetworkMembershipFromCertificate(networkName string, cert *pb.Certificate) (*pb.NetworkMembership, error) {
	id, err := generateSnowflake()
	if err != nil {
		return nil, err
	}

	now := time.Now()
	membership := &pb.NetworkMembership{
		Id:            id,
		CreatedAt:     uint64(now.Unix()),
		Name:          networkName,
		CaCertificate: GetRootCert(cert),
		Certificate:   cert,
		LastSeenAt:    uint64(now.Unix()),
	}

	return membership, nil
}
