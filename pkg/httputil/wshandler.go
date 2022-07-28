// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package httputil

import (
	"context"
	"net/http"
	"time"

	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/gorilla/websocket"
)

var DefaultUpgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type WSHandlerFunc func(c *websocket.Conn)

func (h WSHandlerFunc) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	c, err := DefaultUpgrader.Upgrade(w, r, nil)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	h(c)
}

type WSKeepaliveOptions struct {
	WriteTimeout time.Duration
	ReadTimeout  time.Duration
	PingInterval time.Duration
}

var DefaultWSKeepaliveOptions = WSKeepaliveOptions{
	WriteTimeout: 5 * time.Second,
	ReadTimeout:  25 * time.Second,
	PingInterval: 20 * time.Second,
}

func ScheduleWSKeepalive(ctx context.Context, c *websocket.Conn, opt *WSKeepaliveOptions) {
	if opt == nil {
		opt = &DefaultWSKeepaliveOptions
	}

	c.SetReadDeadline(timeutil.Now().Add(opt.ReadTimeout).Time())
	c.SetPongHandler(func(string) error {
		c.SetReadDeadline(timeutil.Now().Add(opt.ReadTimeout).Time())
		return nil
	})

	timeutil.DefaultTickEmitter.SubscribeCtx(ctx, opt.PingInterval, func(t timeutil.Time) {
		c.WriteControl(websocket.PingMessage, nil, t.Add(opt.WriteTimeout).Time())
	}, nil)
}
