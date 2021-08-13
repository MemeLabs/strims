package control

import (
	"context"
	"io"
	"time"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// BootstrapControl ...
type BootstrapControl interface {
	PublishingEnabled() bool
	Publish(ctx context.Context, peerID uint64, network *networkv1.Network, validDuration time.Duration) error
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
type DirectoryControl interface{}

// NetworkControl ...
type NetworkControl interface {
	Certificate(networkKey []byte) (*certificate.Certificate, bool)
	Add(network *networkv1.Network) error
	Remove(id uint64) error
	ReadEvents(ctx context.Context) <-chan *networkv1.NetworkEvent
	SetDisplayOrder(ids []uint64) error
}

// TransferControl ...
type TransferControl interface {
	Add(swarm *ppspp.Swarm, salt []byte) []byte
	Remove(id []byte)
	List() []*transferv1.Transfer
	Publish(id []byte, networkKey []byte)
}

// VideoCaptureControl ...
type VideoCaptureControl interface {
	Open(mimeType string, directorySnippet *networkv1.DirectoryListingSnippet, networkKeys [][]byte) ([]byte, error)
	OpenWithSwarmWriterOptions(mimeType string, directorySnippet *networkv1.DirectoryListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) ([]byte, error)
	Update(id []byte, directorySnippet *networkv1.DirectoryListingSnippet) error
	Append(id []byte, b []byte, segmentEnd bool) error
	Close(id []byte) error
}

// VideoChannelOption ...
type VideoChannelOption func(channel *videov1.VideoChannel) error

// VideoChannelControl ...
type VideoChannelControl interface {
	GetChannel(id uint64) (*videov1.VideoChannel, error)
	ListChannels() ([]*videov1.VideoChannel, error)
	CreateChannel(opts ...VideoChannelOption) (*videov1.VideoChannel, error)
	UpdateChannel(id uint64, opts ...VideoChannelOption) (*videov1.VideoChannel, error)
	DeleteChannel(id uint64) error
}

// VideoIngressControl ...
type VideoIngressControl interface {
	GetIngressConfig() (*videov1.VideoIngressConfig, error)
	SetIngressConfig(config *videov1.VideoIngressConfig) error
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
	HandlePeerOpen(bindings []*networkv1.NetworkPeerBinding)
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

// VNICControl ...
type VNICControl interface {
	GetConfig() (*vnicv1.Config, error)
	SetConfig(config *vnicv1.Config) error
}

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

type ChatControl interface {
	SyncServer(s *chatv1.Server)
	RemoveServer(id uint64)
	SyncEmote(serverID uint64, e *chatv1.Emote)
	RemoveEmote(id uint64)
	ReadServerEvents(ctx context.Context, networkKey, key []byte) (<-chan *chatv1.ServerEvent, error)
	SendMessage(ctx context.Context, networkKey, key []byte, m string) error
}

// AppControl ...
type AppControl interface {
	Run(ctx context.Context)
	Events() *event.Observers
	Peer() PeerControl
	Bootstrap() BootstrapControl
	CA() CAControl
	Chat() ChatControl
	Dialer() DialerControl
	Directory() DirectoryControl
	Network() NetworkControl
	Transfer() TransferControl
	VideoCapture() VideoCaptureControl
	VideoChannel() VideoChannelControl
	VideoIngress() VideoIngressControl
	VideoEgress() VideoEgressControl
	VNIC() VNICControl
}
