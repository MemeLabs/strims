package dao

import (
	"bytes"
	"strconv"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const networkPrefix = "network:"

func prefixNetworkKey(id uint64) string {
	return networkPrefix + strconv.FormatUint(id, 10)
}

// UpsertNetwork ...
func UpsertNetwork(s kv.RWStore, v *networkv1.Network) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(prefixNetworkKey(v.Id), v)
	})
}

// DeleteNetwork ...
func DeleteNetwork(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(prefixNetworkKey(id))
	})
}

// GetNetwork ...
func GetNetwork(s kv.Store, id uint64) (v *networkv1.Network, err error) {
	v = &networkv1.Network{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNetworkKey(id), v)
	})
	return
}

// GetNetworkByKey ...
func GetNetworkByKey(s kv.Store, key []byte) (*networkv1.Network, error) {
	networks, err := GetNetworks(s)
	if err != nil {
		return nil, err
	}
	for _, v := range networks {
		if bytes.Equal(CertificateRoot(v.Certificate).Key, key) {
			return v, nil
		}
	}
	return nil, kv.ErrRecordNotFound
}

// GetNetworks ...
func GetNetworks(s kv.Store) (v []*networkv1.Network, err error) {
	v = []*networkv1.Network{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(networkPrefix, &v)
	})
	return
}

// NextNetworkDisplayOrder ...
func NextNetworkDisplayOrder(s kv.Store) (n uint32, err error) {
	networks, err := GetNetworks(s)
	for _, v := range networks {
		if v.DisplayOrder >= n {
			n = v.DisplayOrder + 1
		}
	}
	return
}

// NewNetworkCertificate ...
func NewNetworkCertificate(config *networkv1.ServerConfig) (*certificate.Certificate, error) {
	return NewSelfSignedCertificate(config.Key, certificate.KeyUsage_KEY_USAGE_SIGN, defaultCertTTL, WithSubject(config.Name))
}

func SignCertificateRequestWithNetwork(csr *certificate.CertificateRequest, config *networkv1.ServerConfig) (*certificate.Certificate, error) {
	networkCert, err := NewNetworkCertificate(config)
	if err != nil {
		return nil, err
	}

	cert, err := SignCertificateRequest(csr, defaultCertTTL, config.Key)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: networkCert}

	return cert, nil
}

type NewNetworkOptions struct {
	CertificateRequestOptions []CertificateRequestOption
}

type NewNetworkOption func(o *NewNetworkOptions)

func WithCertificateRequestOption(opt CertificateRequestOption) NewNetworkOption {
	return func(o *NewNetworkOptions) {
		o.CertificateRequestOptions = append(o.CertificateRequestOptions, opt)
	}
}

// NewNetwork ...
func NewNetwork(g IDGenerator, name string, icon *networkv1.NetworkIcon, profile *profilev1.Profile, opts ...NewNetworkOption) (*networkv1.Network, error) {
	o := &NewNetworkOptions{
		CertificateRequestOptions: []CertificateRequestOption{
			WithSubject(profile.Name),
		},
	}
	for _, opt := range opts {
		opt(o)
	}

	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	network := &networkv1.Network{
		Id:   id,
		Icon: icon,
		ServerConfigOneof: &networkv1.Network_ServerConfig{
			ServerConfig: &networkv1.ServerConfig{
				Name: name,
				Key:  key,
				Directory: &networkv1directory.ServerConfig{
					Integrations: &networkv1directory.ServerConfig_Integrations{
						Swarm: &networkv1directory.ServerConfig_Integrations_Swarm{
							Enable: true,
						},
					},
				},
			},
		},
	}

	csr, err := NewCertificateRequest(
		profile.Key,
		certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN,
		o.CertificateRequestOptions...,
	)
	if err != nil {
		return nil, err
	}

	cert, err := SignCertificateRequestWithNetwork(csr, network.GetServerConfig())
	if err != nil {
		return nil, err
	}
	network.Certificate = cert

	return network, nil
}

// NewNetworkFromInvitationV0 generates a network from a network invitation
func NewNetworkFromInvitationV0(g IDGenerator, invitation *networkv1.InvitationV0, profile *profilev1.Profile, opts ...NewNetworkOption) (*networkv1.Network, error) {
	o := &NewNetworkOptions{
		CertificateRequestOptions: []CertificateRequestOption{
			WithSubject(profile.Name),
		},
	}
	for _, opt := range opts {
		opt(o)
	}

	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	if err = VerifyCertificate(invitation.Certificate); err != nil {
		return nil, err
	}
	csr, err := NewCertificateRequest(profile.Key, certificate.KeyUsage_KEY_USAGE_PEER, o.CertificateRequestOptions...)
	if err != nil {
		return nil, err
	}
	peerCert, err := SignCertificateRequest(csr, defaultCertTTL, invitation.Key)
	if err != nil {
		return nil, err
	}
	peerCert.ParentOneof = &certificate.Certificate_Parent{Parent: invitation.Certificate}

	return &networkv1.Network{
		Id:          id,
		Certificate: peerCert,
	}, nil
}

// NewNetworkFromCertificate generates a network from a network invitation
func NewNetworkFromCertificate(g IDGenerator, cert *certificate.Certificate) (*networkv1.Network, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	if err = VerifyCertificate(cert); err != nil {
		return nil, err
	}

	return &networkv1.Network{
		Id:          id,
		Certificate: cert,
	}, nil
}

// NewInvitationV0 ...
func NewInvitationV0(key *key.Key, cert *certificate.Certificate) (*networkv1.InvitationV0, error) {
	inviteKey, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	validDuration := time.Hour * 24 * 7 // seven days

	inviteCSR, err := NewCertificateRequest(inviteKey, certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}
	inviteCert, err := SignCertificateRequest(inviteCSR, validDuration, key)
	if err != nil {
		return nil, err
	}
	inviteCert.ParentOneof = &certificate.Certificate_Parent{Parent: cert}

	return &networkv1.InvitationV0{
		Key:         inviteKey,
		Certificate: inviteCert,
	}, nil
}

// NetworkKey ...
func NetworkKey(network *networkv1.Network) []byte {
	return CertificateRoot(network.Certificate).Key
}
