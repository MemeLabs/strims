package control

import (
	"context"
	"io"
	"time"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	transfer "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/api"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// BootstrapControl ...
type BootstrapControl interface {
	PublishingEnabled() bool
	Publish(ctx context.Context, peerID uint64, network *network.Network, validDuration time.Duration) error
}

// CAControl ...
type CAControl interface {
	ForwardRenewRequest(ctx context.Context, cert *certificate.Certificate, csr *certificate.CertificateRequest) (*certificate.Certificate, error)
}

// DialerControl ...
type DialerControl interface {
	ServerDialer(networkKey []byte, key *key.Key, salt []byte) (rpc.Dialer, error)
	Server(networkKey []byte, key *key.Key, salt []byte) (*rpc.Server, error)
	ClientDialer(networkKey, key, salt []byte) (rpc.Dialer, error)
	Client(networkKey, key, salt []byte) (*rpc.Client, error)
}

// DirectoryControl ...
type DirectoryControl interface {
	ReadEvents(ctx context.Context, networkKey []byte) <-chan *network.DirectoryEvent
}

// NetworkControl ...
type NetworkControl interface {
	Certificate(networkKey []byte) (*certificate.Certificate, bool)
	Add(network *network.Network) error
	Remove(id uint64) error
	ReadEvents(ctx context.Context) <-chan *network.NetworkEvent
}

// TransferControl ...
type TransferControl interface {
	Add(swarm *ppspp.Swarm, salt []byte) []byte
	Remove(id []byte)
	List() []*transfer.Transfer
	Publish(id []byte, networkKey []byte)
}

// VideoCaptureControl ...
type VideoCaptureControl interface {
	Open(mimeType string, directorySnippet *network.DirectoryListingSnippet, networkKeys [][]byte) ([]byte, error)
	OpenWithSwarmWriterOptions(mimeType string, directorySnippet *network.DirectoryListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) ([]byte, error)
	Update(id []byte, directorySnippet *network.DirectoryListingSnippet) error
	Append(id []byte, b []byte, segmentEnd bool) error
	Close(id []byte) error
}

// VideoChannelOption ...
type VideoChannelOption func(channel *video.VideoChannel) error

// VideoChannelControl ...
type VideoChannelControl interface {
	GetChannel(id uint64) (*video.VideoChannel, error)
	ListChannels() ([]*video.VideoChannel, error)
	CreateChannel(opts ...VideoChannelOption) (*video.VideoChannel, error)
	UpdateChannel(id uint64, opts ...VideoChannelOption) (*video.VideoChannel, error)
	DeleteChannel(id uint64) error
}

// VideoIngressControl ...
type VideoIngressControl interface {
	GetIngressConfig() (*video.VideoIngressConfig, error)
	SetIngressConfig(config *video.VideoIngressConfig) error
}

// VideoEgressControlBase ...
type VideoEgressControlBase interface {
	OpenStream(swarmURI string, networkKeys [][]byte) ([]byte, io.ReadCloser, error)
}

// VideoHLSEgressControl ...
type VideoHLSEgressControl interface {
	OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error)
	CloseHLSStream(swarmURI string) error
}

// NetworkPeerControl ...
type NetworkPeerControl interface {
	HandlePeerNegotiate(keyCount uint32)
	HandlePeerOpen(bindings []*network.NetworkPeerBinding)
	HandlePeerClose(networkKey []byte)
	HandlePeerUpdateCertificate(cert *certificate.Certificate) error
}

// TransferPeerControl ...
type TransferPeerControl interface {
	AssignPort(id []byte, channel uint64) (uint64, bool)
	Close(id []byte)
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
	VideoCapture() VideoCaptureControl
	VideoChannel() VideoChannelControl
	VideoIngress() VideoIngressControl
	VideoEgress() VideoEgressControl
}
