package notification

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	notificationv1 "github.com/MemeLabs/go-ppspp/pkg/apis/notification/v1"
	"go.uber.org/zap"
)

type Control interface {
	Watch(ctx context.Context) <-chan *notificationv1.Event
	Dispatch(n *notificationv1.Notification) error
	Dismiss(id uint64) error
}

// NewControl ...
func NewControl(logger *zap.Logger, store *dao.ProfileStore, observers *event.Observers) Control {
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
	if err := dao.UpsertNotification(c.store, n); err != nil {
		return err
	}

	c.observers.EmitGlobal(event.Notification{Notification: n})
	return nil
}

func (c *control) Dismiss(id uint64) error {
	if err := dao.DeleteNotification(c.store, id); err != nil {
		return err
	}

	c.observers.EmitGlobal(event.NotificationDismiss{ID: id})
	return nil
}

func (c *control) Watch(ctx context.Context) <-chan *notificationv1.Event {
	ch := make(chan *notificationv1.Event, 8)

	go func() {
		defer close(ch)

		ns, err := dao.GetNotifications(c.store)
		if err != nil {
			c.logger.Debug("loading notifications failed", zap.Error(err))
		}

		for _, n := range ns {
			ch <- &notificationv1.Event{
				Body: &notificationv1.Event_Notification{Notification: n},
			}
		}

		events := make(chan interface{}, 8)
		c.observers.Notify(events)
		defer c.observers.StopNotifying(events)

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case event.Notification:
					ch <- &notificationv1.Event{
						Body: &notificationv1.Event_Notification{
							Notification: e.Notification,
						},
					}
				case event.NotificationDismiss:
					ch <- &notificationv1.Event{
						Body: &notificationv1.Event_Dismiss{
							Dismiss: e.ID,
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
