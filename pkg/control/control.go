package control

import (
	"context"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
)

// BootstrapControl ...
type BootstrapControl interface {
	PublishingEnabled() bool
	Publish(ctx context.Context, peerID uint64, network *pb.Network, validDuration time.Duration) error
}

// CAControl ...
type CAControl interface {
	ForwardRenewRequest(ctx context.Context, cert *pb.Certificate, csr *pb.CertificateRequest) (*pb.Certificate, error)
}

// DialerControl ...
type DialerControl interface {
	ServerDialer(networkKey []byte, key *pb.Key, salt []byte) (rpc.Dialer, error)
	Server(networkKey []byte, key *pb.Key, salt []byte) (*rpc.Server, error)
	ClientDialer(networkKey, key, salt []byte) (rpc.Dialer, error)
	Client(networkKey, key, salt []byte) (*rpc.Client, error)
}

// DirectoryControl ...
type DirectoryControl interface {
	ReadEvents(ctx context.Context, networkKey []byte) <-chan *pb.DirectoryEvent
}

// NetworkControl ...
type NetworkControl interface {
	Certificate(networkKey []byte) (*pb.Certificate, bool)
	Add(network *pb.Network) error
	Remove(id uint64) error
}

// TransferControl ...
type TransferControl interface {
	Add(swarm *ppspp.Swarm, salt []byte) []byte
	Remove(id []byte)
	List() []*pb.Transfer
	Publish(id []byte, networkKey []byte)
}

// VideoChannelOption ...
type VideoChannelOption func(channel *pb.VideoChannel) error

// VideoChannelControl ...
type VideoChannelControl interface {
	GetChannel(id uint64) (*pb.VideoChannel, error)
	ListChannels() ([]*pb.VideoChannel, error)
	CreateChannel(opts ...VideoChannelOption) (*pb.VideoChannel, error)
	UpdateChannel(id uint64, opts ...VideoChannelOption) (*pb.VideoChannel, error)
	DeleteChannel(id uint64) error
}

// VideoIngressControl ...
type VideoIngressControl interface {
	GetIngressConfig() (*pb.VideoIngressConfig, error)
	SetIngressConfig(config *pb.VideoIngressConfig) error
}

// NetworkPeerControl ...
type NetworkPeerControl interface {
	HandlePeerNegotiate(keyCount uint32)
	HandlePeerOpen(bindings []*pb.NetworkPeerBinding)
	HandlePeerClose(networkKey []byte)
	HandlePeerUpdateCertificate(cert *pb.Certificate) error
}

// TransferPeerControl ...
type TransferPeerControl interface {
	AssignPort(id []byte, peerPort uint16) (uint16, bool)
}

// BootstrapPeerControl ...
type BootstrapPeerControl interface{}

// Peer ...
type Peer interface {
	ID() uint64
	Client() api.PeerClient
	VNIC() *vnic.Peer
	Network() NetworkPeerControl
	Transfer() TransferPeerControl
	Bootstrap() BootstrapPeerControl
}

// PeerControl ...
type PeerControl interface {
	Add(peer *vnic.Peer, client api.PeerClient) Peer
	Remove(p Peer)
	Get(id uint64) Peer
	List() []Peer
}

// AppControl ...
type AppControl interface {
	Run(ctx context.Context)
	Peer() PeerControl
	Bootstrap() BootstrapControl
	CA() CAControl
	Dialer() DialerControl
	Directory() DirectoryControl
	Network() NetworkControl
	Transfer() TransferControl
	VideoChannel() VideoChannelControl
	VideoIngress() VideoIngressControl
}
