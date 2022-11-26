package apis

import (
	"github.com/MemeLabs/protobuf/pkg/rpc"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	autoseedv1 "github.com/MemeLabs/strims/pkg/apis/autoseed/v1"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
)

type FrontendClient struct {
	Auth         *authv1.AuthFrontendClient
	Autoseed     *autoseedv1.AutoseedFrontendClient
	Bootstrap    *networkv1bootstrap.BootstrapFrontendClient
	Chat         *chatv1.ChatFrontendClient
	ChatServer   *chatv1.ChatServerFrontendClient
	Debug        *debugv1.DebugClient
	Directory    *networkv1directory.DirectoryFrontendClient
	Network      *networkv1.NetworkFrontendClient
	Notification *notificationv1.NotificationFrontendClient
	Profile      *profilev1.ProfileFrontendClient
	Replication  *replicationv1.ReplicationFrontendClient
	VideoCapture *videov1.CaptureClient
	VideoChannel *videov1.VideoChannelFrontendClient
	VideoEgress  *videov1.EgressClient
	VideoIngress *videov1.VideoIngressClient
	HLSEgress    *videov1.HLSEgressClient
	VNIC         *vnicv1.VNICFrontendClient
}

func NewFrontendClient(c *rpc.Client) *FrontendClient {
	return &FrontendClient{
		Auth:         authv1.NewAuthFrontendClient(c),
		Autoseed:     autoseedv1.NewAutoseedFrontendClient(c),
		Bootstrap:    networkv1bootstrap.NewBootstrapFrontendClient(c),
		Chat:         chatv1.NewChatFrontendClient(c),
		ChatServer:   chatv1.NewChatServerFrontendClient(c),
		Debug:        debugv1.NewDebugClient(c),
		Directory:    networkv1directory.NewDirectoryFrontendClient(c),
		Network:      networkv1.NewNetworkFrontendClient(c),
		Notification: notificationv1.NewNotificationFrontendClient(c),
		Profile:      profilev1.NewProfileFrontendClient(c),
		Replication:  replicationv1.NewReplicationFrontendClient(c),
		VideoCapture: videov1.NewCaptureClient(c),
		VideoChannel: videov1.NewVideoChannelFrontendClient(c),
		VideoEgress:  videov1.NewEgressClient(c),
		VideoIngress: videov1.NewVideoIngressClient(c),
		HLSEgress:    videov1.NewHLSEgressClient(c),
		VNIC:         vnicv1.NewVNICFrontendClient(c),
	}
}
