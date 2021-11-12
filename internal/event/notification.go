package event

import (
	notificationv1 "github.com/MemeLabs/go-ppspp/pkg/apis/notification/v1"
)

// Notification ...
type Notification struct {
	Notification *notificationv1.Notification
}

// NotificationDismiss ...
type NotificationDismiss struct {
	ID uint64
}
