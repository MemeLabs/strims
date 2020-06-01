package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"strconv"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

var nextSnowflakeID uint64

// generate a 53 bit locally unique id
func generateSnowflake() (uint64, error) {
	seconds := uint64(time.Since(time.Date(2020, 0, 0, 0, 0, 0, 0, time.UTC)) / time.Second)
	sequence := atomic.AddUint64(&nextSnowflakeID, 1) << 32
	return (seconds | sequence) & 0x1fffffffffffff, nil
}

func GenerateKey() (*pb.Key, error) {
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	k := &pb.Key{
		Type:    pb.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  pub,
	}
	return k, nil
}

const profileKeyPrefix = "profile:"
const profileSummaryKeyPrefix = "profileSummary:"
const networkPrefix = "network:"
const networkMembershipPrefix = "networkMembership:"
const bootstrapClientPrefix = "bootstrapClient:"
const chatServerPrefix = "chatServer:"

func prefixProfileKey(id uint64) string {
	return profileKeyPrefix + strconv.FormatUint(id, 10)
}

func prefixProfileSummaryKey(name string) string {
	return profileSummaryKeyPrefix + name
}

func prefixNetworkKey(id uint64) string {
	return networkPrefix + strconv.FormatUint(id, 10)
}

func prefixNetworkMembershipKey(id uint64) string {
	return networkMembershipPrefix + strconv.FormatUint(id, 10)
}

func prefixBootstrapClientKey(id uint64) string {
	return bootstrapClientPrefix + strconv.FormatUint(id, 10)
}

func prefixChatServerKey(id uint64) string {
	return chatServerPrefix + strconv.FormatUint(id, 10)
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

// NewProfileSummary ...
func NewProfileSummary(p *pb.Profile) *pb.ProfileSummary {
	return &pb.ProfileSummary{
		Id:   p.Id,
		Name: p.Name,
	}
}

// NewNetwork ...
func NewNetwork(name string) (*pb.Network, error) {
	id, err := generateSnowflake()
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
	csr, err := NewCertificateRequest(key, pb.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}

	network.Certificate, err = SignCertificateRequest(csr, defaultCertTTL, key)
	if err != nil {
		return nil, err
	}
	return network, nil
}

// NewNetworkMembershipFromNetwork generates a network membership from a network
func NewNetworkMembershipFromNetwork(network *pb.Network, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	return NewNetworkMembership(network.Name, network.Certificate, network.Certificate, network.Key, csr)
}

// NewNetworkMembershipFromInvite generates a network membership from a network invitation
func NewNetworkMembershipFromInvite(invite *pb.InvitationV0, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	networkCert := GetRootCert(invite.Certificate)

	signingKey := &pb.Key{
		Private: invite.Key,
		Type:    pb.KeyType_KEY_TYPE_ED25519,
	}

	return NewNetworkMembership(invite.NetworkName, networkCert, invite.Certificate, signingKey, csr)
}

// NewNetworkMembership generates a new network membership using the given parameters
func NewNetworkMembership(networkName string, networkCert *pb.Certificate, parentCert *pb.Certificate, csrSigningKey *pb.Key, csr *pb.CertificateRequest) (*pb.NetworkMembership, error) {
	id, err := generateSnowflake()
	if err != nil {
		return nil, err
	}

	now := time.Now()
	membership := &pb.NetworkMembership{
		Id:            id,
		CreatedAt:     uint64(now.Unix()),
		Name:          networkName,
		CaCertificate: networkCert,
		LastSeenAt:    uint64(now.Unix()),
	}

	membership.Certificate, err = SignCertificateRequest(csr, defaultCertTTL, csrSigningKey)
	if err != nil {
		return nil, err
	}
	membership.Certificate.ParentOneof = &pb.Certificate_Parent{Parent: parentCert}

	return membership, nil
}

// NewWebSocketBootstrapClient ...
func NewWebSocketBootstrapClient(url string) (*pb.BootstrapClient, error) {
	id, err := generateSnowflake()
	if err != nil {
		return nil, err
	}

	return &pb.BootstrapClient{
		Id: id,
		ClientOptions: &pb.BootstrapClient_WebsocketOptions{
			WebsocketOptions: &pb.BootstrapClientWebSocketOptions{
				Url: url,
			},
		},
	}, nil
}

// NewChatServer ...
func NewChatServer(networkKey []byte, chatRoom *pb.ChatRoom) (*pb.ChatServer, error) {
	id, err := generateSnowflake()
	if err != nil {
		return nil, err
	}

	network := &pb.ChatServer{
		Id:         id,
		NetworkKey: networkKey,
		ChatRoom:   chatRoom,
	}
	return network, nil
}
