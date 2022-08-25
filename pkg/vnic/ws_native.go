// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package vnic

import (
	"context"
	"crypto/tls"
	"fmt"
	"net"
	"net/http"
	"net/netip"
	"net/url"
	"strconv"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func init() { RegisterLinkInterface("ws", (*wsLinkCandidate)(nil)) }

var _ Interface = (*wsInterface)(nil)
var _ LinkDialer = (*wsInterface)(nil)
var _ LinkCandidate = (*wsLinkCandidate)(nil)

type WSInterfaceOptions struct {
	ServeMux       *httputil.MapServeMux
	Address        string
	PublicHostname string
	PublicPort     uint16
	Secure         bool
	AllowInsecure  bool
	ConnOptions    httputil.WSOptions
}

// NewWSInterface ...
func NewWSInterface(logger *zap.Logger, options WSInterfaceOptions) Interface {
	return &wsInterface{
		logger:  logger,
		options: options,
	}
}

type wsInterface struct {
	logger  *zap.Logger
	options WSInterfaceOptions
	path    string
	uri     string
}

func (f *wsInterface) ValidScheme(scheme string) bool {
	return scheme == "wss" || (f.options.AllowInsecure && scheme == "ws")
}

func (f *wsInterface) Listen(h *Host) error {
	if f.options.ServeMux == nil {
		return nil
	}

	f.path = fmt.Sprintf("/%x", h.profileKey.Public)
	if u, err := f.formatURI(); err != nil {
		f.logger.Debug("failed to format ws uri", zap.Error(err))
	} else {
		f.uri = u
	}

	f.logger.Debug("ws vnic listener starting", zap.String("uri", f.uri))
	f.options.ServeMux.HandleWSFunc(f.path, func(c *websocket.Conn) {
		h.AddLink(httputil.NewWSReadWriter(c, f.options.ConnOptions))
	})
	return nil
}

func (f *wsInterface) formatURI() (string, error) {
	u := &url.URL{
		Scheme: "ws",
		Path:   f.path,
	}

	if f.options.Secure {
		u.Scheme = "wss"
	}

	host, port, err := net.SplitHostPort(f.options.Address)
	if err != nil {
		return "", err
	}
	if p := f.options.PublicPort; p != 0 {
		port = strconv.Itoa(int(p))
	}
	if h := f.options.PublicHostname; h != "" {
		host = h
	} else {
		a, err := netip.ParseAddr(host)
		if (host == "" || err == nil) && (a.IsUnspecified() || !a.IsValid()) {
			return "", fmt.Errorf("invalid hostname: %s", host)
		}
	}
	u.Host = net.JoinHostPort(host, port)

	return u.String(), nil
}

func (f *wsInterface) Close() error {
	if f.options.ServeMux != nil {
		f.options.ServeMux.StopHandling(f.path)
	}
	return nil
}

func (f *wsInterface) Dial(uri string) (Link, error) {
	u, err := url.Parse(uri)
	if err != nil {
		return nil, err
	}

	d := &websocket.Dialer{
		TLSClientConfig: &tls.Config{
			InsecureSkipVerify: u.Fragment == "insecure",
		},
	}
	c, _, err := d.Dial(uri, http.Header{})
	if err != nil {
		return nil, err
	}
	return httputil.NewWSReadWriter(c, f.options.ConnOptions), nil
}

func (f *wsInterface) CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error) {
	return &wsLinkCandidate{f, h}, nil
}

type wsLinkCandidate struct {
	iface *wsInterface
	host  *Host
}

func (f *wsLinkCandidate) LocalDescription() (*vnicv1.LinkDescription, error) {
	if f.iface.uri == "" {
		return nil, nil
	}

	d := &vnicv1.LinkDescription{
		Interface:   "ws",
		Description: f.iface.uri,
	}
	return d, nil
}

func (f *wsLinkCandidate) SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error) {
	err := f.host.Dial(d.Description)
	return err == nil, err
}
