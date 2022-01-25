package dao

import (
	"time"

	notificationv1 "github.com/MemeLabs/go-ppspp/pkg/apis/notification/v1"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + notificationNS
	notificationNotificationNS
)

var Notifications = NewTable(
	notificationNotificationNS,
	&TableOptions[notificationv1.Notification]{
		ObserveChange: func(m, p *notificationv1.Notification) proto.Message {
			return &notificationv1.NotificationChangeEvent{Notification: m}
		},
		ObserveDelete: func(m *notificationv1.Notification) proto.Message {
			return &notificationv1.NotificationDeleteEvent{Notification: m}
		},
	},
)

type NewNotificationOptions struct {
	Message string
	Subject *notificationv1.Notification_Subject
}

type NewNotificationOption func(o *NewNotificationOptions)

func WithNotificationMessage(message string) NewNotificationOption {
	return func(o *NewNotificationOptions) {
		o.Message = message
	}
}

func WithNotificationSubject(model notificationv1.Notification_Subject_Model, id uint64) NewNotificationOption {
	return func(o *NewNotificationOptions) {
		o.Subject = &notificationv1.Notification_Subject{
			Model: model,
			Id:    id,
		}
	}
}

// NewNotification ...
func NewNotification(g IDGenerator, status notificationv1.Notification_Status, title string, opts ...NewNotificationOption) (*notificationv1.Notification, error) {
	o := &NewNotificationOptions{}
	for _, opt := range opts {
		opt(o)
	}

	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	notification := &notificationv1.Notification{
		Id:        id,
		CreatedAt: timeutil.Now().UnixNano() / int64(time.Millisecond),
		Status:    status,
		Title:     title,
		Message:   o.Message,
	}

	return notification, nil
}
