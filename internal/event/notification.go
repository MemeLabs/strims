// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
)

// Notification ...
type Notification struct {
	Notification *notificationv1.Notification
}

// NotificationDismiss ...
type NotificationDismiss struct {
	ID uint64
}
