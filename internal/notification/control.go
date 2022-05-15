// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package notification

import (
	"context"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
	"go.uber.org/zap"
)

type Control interface {
	Watch(ctx context.Context) <-chan *notificationv1.Event
	Dispatch(n *notificationv1.Notification) error
	Dismiss(id uint64) error
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
) Control {
	return &control{
		logger:    logger,
		store:     store,
		observers: observers,
	}
}

type control struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
}

func (c *control) Dispatch(n *notificationv1.Notification) error {
	return dao.Notifications.Upsert(c.store, n)
}

func (c *control) Dismiss(id uint64) error {
	return dao.Notifications.Delete(c.store, id)
}

func (c *control) Watch(ctx context.Context) <-chan *notificationv1.Event {
	ch := make(chan *notificationv1.Event, 8)

	go func() {
		defer close(ch)

		ns, err := dao.Notifications.GetAll(c.store)
		if err != nil {
			c.logger.Debug("loading notifications failed", zap.Error(err))
		}

		for _, n := range ns {
			ch <- &notificationv1.Event{
				Body: &notificationv1.Event_Notification{Notification: n},
			}
		}

		events, done := c.observers.Events()
		defer done()

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case *notificationv1.NotificationChangeEvent:
					ch <- &notificationv1.Event{
						Body: &notificationv1.Event_Notification{
							Notification: e.Notification,
						},
					}
				case *notificationv1.NotificationDeleteEvent:
					ch <- &notificationv1.Event{
						Body: &notificationv1.Event_Dismiss{
							Dismiss: e.Notification.Id,
						},
					}
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch
}
