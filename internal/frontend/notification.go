// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		notificationv1.RegisterNotificationFrontendService(server, &notificataionService{
			store: params.Store,
			app:   params.App,
		})
	})
}

// notificataionService ...
type notificataionService struct {
	store *dao.ProfileStore
	app   app.Control
}

// Dismiss ...
func (s *notificataionService) Dismiss(ctx context.Context, r *notificationv1.DismissRequest) (*notificationv1.DismissResponse, error) {
	for _, id := range r.Ids {
		s.app.Notification().Dismiss(id)
	}
	return &notificationv1.DismissResponse{}, nil
}

// Watch ...
func (s *notificataionService) Watch(ctx context.Context, r *notificationv1.WatchRequest) (<-chan *notificationv1.WatchResponse, error) {
	ch := make(chan *notificationv1.WatchResponse, 1)

	go func() {
		for e := range s.app.Notification().Watch(ctx) {
			ch <- &notificationv1.WatchResponse{Event: e}
		}
	}()

	return ch, nil
}
