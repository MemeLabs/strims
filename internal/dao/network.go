package dao

import (
	"bytes"
	"encoding/binary"
	"errors"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + networkNS
	networkNetworkNS
	networkNetworkKeyNS
	networkCertificateLogNS
	networkCertificateLogNetworkNS
	networkCertificateLogSerialNS
	networkCertificateLogSubjectNS
	networkBootstrapClientNS
	networkUIConfigNS
	networkPeerNS
	networkPeerNetworkNS
	networkPeerPublicKeyNS
	networkPeerInviterNS
)

var Networks = NewTable(
	networkNetworkNS,
	&TableOptions[networkv1.Network, *networkv1.Network]{
		ObserveChange: func(m, p *networkv1.Network) proto.Message {
			return &networkv1.NetworkChangeEvent{Network: m}
		},
		ObserveDelete: func(m *networkv1.Network) proto.Message {
			return &networkv1.NetworkDeleteEvent{Network: m}
		},
	},
)

var GetNetworkByKey = SecondaryIndex(networkNetworkKeyNS, Networks, NetworkKey)

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
	Alias                     string
}

type NewNetworkOption func(o *NewNetworkOptions)

func WithCertificateRequestOption(opt CertificateRequestOption) NewNetworkOption {
	return func(o *NewNetworkOptions) {
		o.CertificateRequestOptions = append(o.CertificateRequestOptions, opt)
	}
}

func WithAlias(alias string) NewNetworkOption {
	return func(o *NewNetworkOptions) {
		if alias != "" {
			o.Alias = alias
			o.CertificateRequestOptions = append(o.CertificateRequestOptions, WithSubject(alias))
		}
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
		Alias:       o.Alias,
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

var CertificateLogs = NewTable[networkv1ca.CertificateLog](networkCertificateLogNS, nil)

var GetCertificateLogsByNetworkID, GetCertificateLogsByNetwork, GetNetworkByCertificateLog = ManyToOne(
	networkCertificateLogNetworkNS,
	CertificateLogs,
	Networks,
	(*networkv1ca.CertificateLog).GetNetworkId,
	&ManyToOneOptions{CascadeDelete: true},
)

func FormatCertificateLogSerialNumberKey(networkID uint64, serialNumber []byte) []byte {
	b := make([]byte, 8, 8+len(serialNumber))
	binary.BigEndian.PutUint64(b, networkID)
	return append(b, serialNumber...)
}

func certificateLogSerialNumberKey(m *networkv1ca.CertificateLog) []byte {
	return FormatCertificateLogSerialNumberKey(m.NetworkId, m.Certificate.SerialNumber)
}

var GetCertificateLogBySerialNumber = UniqueIndex(
	networkCertificateLogSerialNS,
	CertificateLogs,
	certificateLogSerialNumberKey,
	nil,
)

func FormatCertificateLogSubjectKey(networkID uint64, subject string) []byte {
	b := make([]byte, 8, 8+len([]byte(subject)))
	binary.BigEndian.PutUint64(b, networkID)
	return append(b, []byte(subject)...)
}

func certificateLogSubjectKey(m *networkv1ca.CertificateLog) []byte {
	return FormatCertificateLogSubjectKey(m.NetworkId, m.Certificate.Subject)
}

var ErrCertificateSubjectInUse = errors.New("certificate subject in use")

var GetCertificateLogBySubject = UniqueIndex(
	networkCertificateLogSubjectNS,
	CertificateLogs,
	certificateLogSubjectKey,
	&UniqueIndexOptions[networkv1ca.CertificateLog, *networkv1ca.CertificateLog]{
		OnConflict: func(s kv.RWStore, t *Table[networkv1ca.CertificateLog, *networkv1ca.CertificateLog], m, p *networkv1ca.CertificateLog) error {
			if bytes.Equal(m.Certificate.Key, p.Certificate.Key) {
				return DeleteSecondaryIndex(s, networkCertificateLogSubjectNS, certificateLogSubjectKey(m), p.Id)
			}
			return ErrCertificateSubjectInUse
		},
	},
)

func FormatCertificateLogKeyKey(networkID uint64, key []byte) []byte {
	b := make([]byte, 8, 8+len([]byte(key)))
	binary.BigEndian.PutUint64(b, networkID)
	return append(b, []byte(key)...)
}

func certificateLogKeyKey(m *networkv1ca.CertificateLog) []byte {
	return FormatCertificateLogKeyKey(m.NetworkId, m.Certificate.Key)
}

var GetCertificateLogByKey = UniqueIndex(
	networkCertificateLogSubjectNS,
	CertificateLogs,
	certificateLogKeyKey,
	&UniqueIndexOptions[networkv1ca.CertificateLog, *networkv1ca.CertificateLog]{
		OnConflict: func(s kv.RWStore, t *Table[networkv1ca.CertificateLog, *networkv1ca.CertificateLog], m, p *networkv1ca.CertificateLog) error {
			if bytes.Equal(m.Certificate.Key, p.Certificate.Key) {
				return DeleteSecondaryIndex(s, networkCertificateLogSubjectNS, certificateLogSubjectKey(m), p.Id)
			}
			return ErrCertificateSubjectInUse
		},
	},
)

// NewCertificateLog ...
func NewCertificateLog(s IDGenerator, networkID uint64, cert *certificate.Certificate) (*networkv1ca.CertificateLog, error) {
	id, err := s.GenerateID()
	if err != nil {
		return nil, err
	}

	c := proto.Clone(cert).(*certificate.Certificate)
	if p := c.GetParent(); p != nil {
		c.ParentOneof = &certificate.Certificate_ParentSerialNumber{
			ParentSerialNumber: p.SerialNumber,
		}
	}

	return &networkv1ca.CertificateLog{
		Id:          id,
		NetworkId:   networkID,
		Certificate: c,
	}, nil
}

func NewCertificateLogCache(s kv.RWStore, opt *CacheStoreOptions) (c CertificateLogCache) {
	c.CacheStore, c.ByID = newCacheStore[networkv1ca.CertificateLog](s, CertificateLogs, opt)
	c.BySerialNumber = NewCacheIndex(
		c.CacheStore,
		GetCertificateLogBySerialNumber,
		certificateLogSerialNumberKey,
		hashmap.NewByteInterface[[]byte],
	)
	c.BySubject = NewCacheIndex(
		c.CacheStore,
		GetCertificateLogBySubject,
		certificateLogSubjectKey,
		hashmap.NewByteInterface[[]byte],
	)
	c.ByKey = NewCacheIndex(
		c.CacheStore,
		GetCertificateLogByKey,
		certificateLogKeyKey,
		hashmap.NewByteInterface[[]byte],
	)
	return
}

type CertificateLogCache struct {
	*CacheStore[networkv1ca.CertificateLog, *networkv1ca.CertificateLog]
	ByID           CacheAccessor[uint64, networkv1ca.CertificateLog, *networkv1ca.CertificateLog]
	BySerialNumber CacheAccessor[[]byte, networkv1ca.CertificateLog, *networkv1ca.CertificateLog]
	BySubject      CacheAccessor[[]byte, networkv1ca.CertificateLog, *networkv1ca.CertificateLog]
	ByKey          CacheAccessor[[]byte, networkv1ca.CertificateLog, *networkv1ca.CertificateLog]
}

// NetworkKey ...
func NetworkKey(network *networkv1.Network) []byte {
	return CertificateRoot(network.Certificate).Key
}

var BootstrapClients = NewTable(
	networkBootstrapClientNS,
	&TableOptions[networkv1bootstrap.BootstrapClient, *networkv1bootstrap.BootstrapClient]{
		ObserveChange: func(m, p *networkv1bootstrap.BootstrapClient) proto.Message {
			return &networkv1bootstrap.BootstrapClientChange{BootstrapClient: m}
		},
	},
)

// NewWebSocketBootstrapClient ...
func NewWebSocketBootstrapClient(g IDGenerator, url string, insecureSkipVerifyTLS bool) (*networkv1bootstrap.BootstrapClient, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	return &networkv1bootstrap.BootstrapClient{
		Id: id,
		ClientOptions: &networkv1bootstrap.BootstrapClient_WebsocketOptions{
			WebsocketOptions: &networkv1bootstrap.BootstrapClientWebSocketOptions{
				Url:                   url,
				InsecureSkipVerifyTls: insecureSkipVerifyTLS,
			},
		},
	}, nil
}

var NetworkUIConfig = NewSingleton(
	networkUIConfigNS,
	&SingletonOptions[networkv1.UIConfig, *networkv1.UIConfig]{
		DefaultValue: &networkv1.UIConfig{
			NetworkDisplayOrder: []uint64{},
		},
		ObserveChange: func(m, p *networkv1.UIConfig) proto.Message {
			return &networkv1.UIConfigChangeEvent{UiConfig: m}
		},
	},
)

var NetworkPeers = NewTable[networkv1.Peer](networkPeerNS, nil)

var GetNetworkPeersByNetworkID, GetNetworkPeersByNetwork, GetNetworkByNetworkPeer = ManyToOne(
	networkPeerNetworkNS,
	NetworkPeers,
	Networks,
	(*networkv1.Peer).GetNetworkId,
	&ManyToOneOptions{CascadeDelete: true},
)

var GetNetworkPeersByInviterPeerID, GetNetworkPeersByInviterPeer, GetInviterPeerByNetworkPeer = ManyToOne(
	networkPeerInviterNS,
	NetworkPeers,
	NetworkPeers,
	(*networkv1.Peer).GetInviterPeerId,
	nil,
)

func FormatNetworkPeerPublicKeyKey(networkID uint64, key []byte) []byte {
	b := make([]byte, 8, 8+len(key))
	binary.BigEndian.PutUint64(b, networkID)
	return append(b, key...)
}

var GetNetworkPeerByPublicKey = UniqueIndex(
	networkPeerPublicKeyNS,
	NetworkPeers,
	func(m *networkv1.Peer) []byte {
		return FormatNetworkPeerPublicKeyKey(m.NetworkId, m.PublicKey)
	},
	nil,
)

// NewNetworkPeer ...
func NewNetworkPeer(g IDGenerator, networkID uint64, publicKey []byte, inviterPeerID uint64) (*networkv1.Peer, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	return &networkv1.Peer{
		Id:            id,
		NetworkId:     networkID,
		PublicKey:     publicKey,
		InviterPeerId: inviterPeerID,
	}, nil
}

func GetOrCreateNetworkPeer(s *ProfileStore, networkID uint64, publicKey []byte, inviterPeerID uint64) (*networkv1.Peer, error) {
	for retries := 0; retries < 2; retries++ {
		p, err := GetNetworkPeerByPublicKey(s, publicKey)
		if err == nil || !errors.Is(err, kv.ErrRecordNotFound) {
			return p, err
		}

		p, err = NewNetworkPeer(s, networkID, publicKey, inviterPeerID)
		if err != nil {
			return nil, err
		}

		err = NetworkPeers.Insert(s, p)
		if err == nil {
			return p, nil
		} else if !errors.Is(err, ErrUniqueConstraintViolated) {
			return nil, err
		}
	}
	return nil, errors.New("unexpected error creating peer record")
}
