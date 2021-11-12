package notificationv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterNotificationFrontendService ...
func RegisterNotificationFrontendService(host rpc.ServiceRegistry, service NotificationFrontendService) {
	host.RegisterMethod("strims.notification.v1.NotificationFrontend.Watch", service.Watch)
	host.RegisterMethod("strims.notification.v1.NotificationFrontend.Dismiss", service.Dismiss)
}

// NotificationFrontendService ...
type NotificationFrontendService interface {
	Watch(
		ctx context.Context,
		req *WatchRequest,
	) (<-chan *WatchResponse, error)
	Dismiss(
		ctx context.Context,
		req *DismissRequest,
	) (*DismissResponse, error)
}

// NotificationFrontendClient ...
type NotificationFrontendClient struct {
	client rpc.Caller
}

// NewNotificationFrontendClient ...
func NewNotificationFrontendClient(client rpc.Caller) *NotificationFrontendClient {
	return &NotificationFrontendClient{client}
}

// Watch ...
func (c *NotificationFrontendClient) Watch(
	ctx context.Context,
	req *WatchRequest,
	res chan *WatchResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.notification.v1.NotificationFrontend.Watch", req, res)
}

// Dismiss ...
func (c *NotificationFrontendClient) Dismiss(
	ctx context.Context,
	req *DismissRequest,
	res *DismissResponse,
) error {
	return c.client.CallUnary(ctx, "strims.notification.v1.NotificationFrontend.Dismiss", req, res)
}
