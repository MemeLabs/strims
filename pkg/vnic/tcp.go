// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package vnic

import (
	"context"
	"fmt"
	"log"
	"math"
	"net"
	"net/netip"
	"net/url"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"go.uber.org/zap"
)

func init() { RegisterLinkInterface("tcp", (*tcpLinkCandidate)(nil)) }

var _ Interface = (*tcpInterface)(nil)
var _ LinkDialer = (*tcpInterface)(nil)
var _ LinkCandidate = (*tcpLinkCandidate)(nil)

type TCPInterfaceOptions struct {
	Address string
	HostIP  string
}

// NewTCPInterface ...
func NewTCPInterface(logger *zap.Logger, options TCPInterfaceOptions) Interface {
	return &tcpInterface{
		logger:  logger,
		options: options,
	}
}

type tcpInterface struct {
	logger   *zap.Logger
	options  TCPInterfaceOptions
	path     string
	uri      string
	listener *net.TCPListener
}

func (f *tcpInterface) ValidScheme(scheme string) bool {
	return scheme == "tcp"
}

func (f *tcpInterface) Listen(h *Host) error {
	// TODO: mux
	if f.options.Address == "" {
		return nil
	}

	f.path = fmt.Sprintf("/%x", h.profileKey.Public)
	if u, err := f.formatURI(); err != nil {
		f.logger.Debug("failed to format tcp uri", zap.Error(err))
	} else {
		f.uri = u
	}

	f.logger.Debug("tcp vnic listener starting", zap.String("uri", f.uri))

	addr, err := net.ResolveTCPAddr("tcp", f.options.Address)
	if err != nil {
		return err
	}
	f.listener, err = net.ListenTCP("tcp", addr)
	if err != nil {
		return err
	}

	for {
		c, err := f.listener.AcceptTCP()
		if err != nil {
			log.Println("tcp accept err", err)
			return err
		}

		log.Println("got tcp conn", c)

		// TODO: handshake

		h.AddLink(tcpConn{c})
	}
}

func (f *tcpInterface) formatURI() (string, error) {
	ap, err := netip.ParseAddrPort(f.options.Address)
	if err != nil {
		return "", err
	}

	if f.options.HostIP != "" {
		a, err := netip.ParseAddr(f.options.HostIP)
		if err != nil {
			return "", err
		}
		ap = netip.AddrPortFrom(a, ap.Port())
	}

	if ap.Addr().IsUnspecified() || !ap.Addr().IsValid() {
		return "", fmt.Errorf("invalid ip: %s", ap.Addr())
	}

	u := url.URL{
		Scheme: "tcp",
		Host:   ap.String(),
		Path:   f.path,
	}
	return u.String(), nil
}

func (f *tcpInterface) Close() error {
	return f.listener.Close()
}

func (f *tcpInterface) Dial(uri string) (Link, error) {
	u, err := url.Parse(uri)
	if err != nil {
		return nil, err
	}

	a, err := net.ResolveTCPAddr("tcp", u.Host)
	if err != nil {
		return nil, err
	}

	c, err := net.DialTCP("tcp", nil, a)
	if err != nil {
		return nil, err
	}

	// TODO: handshake

	return tcpConn{c}, nil
}

func (f *tcpInterface) CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error) {
	return &tcpLinkCandidate{f, h}, nil
}

type tcpLinkCandidate struct {
	iface *tcpInterface
	host  *Host
}

func (f *tcpLinkCandidate) LocalDescription() (*vnicv1.LinkDescription, error) {
	if f.iface.uri == "" {
		return nil, nil
	}

	d := &vnicv1.LinkDescription{
		Interface:   "tcp",
		Description: f.iface.uri,
	}
	return d, nil
}

func (f *tcpLinkCandidate) SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error) {
	err := f.host.Dial(d.Description)
	return err == nil, err
}

type tcpConn struct {
	*net.TCPConn
}

func (c tcpConn) MTU() int {
	return math.MaxUint16
}
