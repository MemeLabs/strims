// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package vnic

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"math"
	"net"
	"net/netip"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/ed25519util"
	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"go.uber.org/zap"
)

func init() { RegisterLinkInterface("tcp", (*tcpLinkCandidate)(nil)) }

var _ Interface = (*tcpInterface)(nil)
var _ LinkDialer = (*tcpInterface)(nil)
var _ LinkCandidate = (*tcpLinkCandidate)(nil)

type TCPInterfaceOptions struct {
	Address         string
	HostIP          string
	Mux             *TCPMux
	KeepAlivePeriod time.Duration
	ReadBufferSize  int
	WriteBufferSize int
	ReadTimeout     time.Duration
	WriteTimeout    time.Duration
}

var DefaultTCPInterfaceOptions = TCPInterfaceOptions{
	KeepAlivePeriod: 20 * time.Second,
	ReadBufferSize:  2 * 1024 * 1024,
	WriteBufferSize: 2 * 1024 * 1024,
	ReadTimeout:     25 * time.Second,
	WriteTimeout:    5 * time.Second,
}

// NewTCPInterface ...
func NewTCPInterface(logger *zap.Logger, o TCPInterfaceOptions) Interface {
	o = options.AssignDefaults(o, DefaultTCPInterfaceOptions)

	return &tcpInterface{
		logger:  logger,
		options: o,
	}
}

type tcpInterface struct {
	logger  *zap.Logger
	options TCPInterfaceOptions
	key     *key.Key
	peerKey []byte
	uri     string
}

func (f *tcpInterface) ValidScheme(scheme string) bool {
	return scheme == "tcp"
}

func (f *tcpInterface) Listen(h *Host) error {
	f.key = ed25519util.KeyToCurve25519(h.profileKey)

	if f.options.Mux == nil {
		return nil
	}

	f.peerKey = h.profileKey.Public
	if u, err := f.formatURI(); err != nil {
		f.logger.Debug("failed to format tcp uri", zap.Error(err))
	} else {
		f.uri = u
	}

	f.logger.Debug("tcp vnic listener starting", zap.String("uri", f.uri))
	f.options.Mux.Handle(f.peerKey, TCPConnHandlerFunc(func(c *net.TCPConn) error {
		l, err := f.createTCPConn(c)
		if err != nil {
			return err
		}

		l, err = handshakeAESLink(l, f.key, nil)
		if err != nil {
			return err
		}

		h.AddLink(l)
		return nil
	}))
	return nil
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
		Path:   fmt.Sprintf("/%x", f.peerKey),
	}
	return u.String(), nil
}

func (f *tcpInterface) Close() error {
	if f.options.Mux != nil {
		f.options.Mux.StopHandling(f.peerKey)
	}
	return nil
}

func (f *tcpInterface) Dial(uri string) (Link, error) {
	u, err := url.Parse(uri)
	if err != nil {
		return nil, err
	}

	peerKey, err := hex.DecodeString(strings.TrimLeft(u.Path, "/"))
	if err != nil {
		return nil, err
	}
	if len(peerKey) != 32 {
		return nil, errors.New("invalid peer key size")
	}

	a, err := net.ResolveTCPAddr("tcp", u.Host)
	if err != nil {
		return nil, err
	}

	c, err := DialTCPMux(a, peerKey)
	if err != nil {
		return nil, err
	}

	l, err := f.createTCPConn(c)
	if err != nil {
		return nil, err
	}

	var peerCurve25519Key [32]byte
	ed25519util.PublicKeyToCurve25519(&peerCurve25519Key, (*[32]byte)(peerKey))
	return handshakeAESLink(l, f.key, peerCurve25519Key[:])
}

func (f *tcpInterface) createTCPConn(c *net.TCPConn) (Link, error) {
	if err := c.SetKeepAlive(true); err != nil {
		return nil, err
	}
	if err := c.SetKeepAlivePeriod(f.options.KeepAlivePeriod); err != nil {
		return nil, err
	}
	if err := c.SetReadBuffer(f.options.ReadBufferSize); err != nil {
		return nil, err
	}
	if err := c.SetWriteBuffer(f.options.WriteBufferSize); err != nil {
		return nil, err
	}

	l := &tcpConn{
		ReadTimeout:  f.options.ReadTimeout,
		WriteTimeout: f.options.WriteTimeout,
		TCPConn:      c,
	}
	return l, nil
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
	_, err := f.host.Dial(d.Description)
	return err == nil, err
}

func NewTCPMux(logger *zap.Logger, addr string) (*TCPMux, *net.TCPListener, error) {
	m := &TCPMux{logger: logger}
	l, err := m.Listen(addr)
	if err != nil {
		return nil, nil, err
	}
	return m, l, nil
}

type TCPMux struct {
	logger   *zap.Logger
	handlers syncutil.Map[[32]byte, TCPConnHandler]
}

func (m *TCPMux) Handle(k []byte, h TCPConnHandler) {
	m.handlers.Set(*(*[32]byte)(k), h)
}

func (m *TCPMux) StopHandling(k []byte) {
	m.handlers.Delete(*(*[32]byte)(k))
}

func (m *TCPMux) Listen(addr string) (*net.TCPListener, error) {
	a, err := net.ResolveTCPAddr("tcp", addr)
	if err != nil {
		return nil, err
	}
	l, err := net.ListenTCP("tcp", a)
	if err != nil {
		return nil, err
	}

	go func() {
		for {
			c, err := l.AcceptTCP()
			if err != nil {
				m.logger.Debug("tcp listener closed with error", zap.Error(err))
				return
			}

			go func() {
				if err := m.handleConn(c); err != nil {
					m.logger.Debug("mux connection handler failed", zap.Error(err))
					c.Close()
				}
			}()
		}
	}()

	return l, nil
}

func (m *TCPMux) handleConn(c *net.TCPConn) (err error) {
	var init vnicv1.TCPMuxInit
	if err := protoutil.ReadStream(c, &init); err != nil {
		return fmt.Errorf("reading peer init failed: %w", err)
	}

	if len(init.PeerKey) != 32 {
		return errors.New("invalid peer key size")
	}
	h, ok := m.handlers.Get(*(*[32]byte)(init.PeerKey))
	if !ok {
		return errors.New("peer key not found")
	}

	return h.HandleConn(c)
}

func DialTCPMux(a *net.TCPAddr, peerKey []byte) (*net.TCPConn, error) {
	c, err := net.DialTCP("tcp", nil, a)
	if err != nil {
		return nil, err
	}

	err = protoutil.WriteStream(c, &vnicv1.TCPMuxInit{
		ProtocolVersion: 1,
		PeerKey:         peerKey,
	})
	if err != nil {
		return nil, fmt.Errorf("writing tcp mux init failed: %w", err)
	}

	return c, nil
}

type TCPConnHandler interface {
	HandleConn(c *net.TCPConn) error
}

type TCPConnHandlerFunc func(*net.TCPConn) error

func (f TCPConnHandlerFunc) HandleConn(l *net.TCPConn) error {
	return f(l)
}

type tcpConn struct {
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
	writeLock    sync.Mutex
	*net.TCPConn
}

func (c *tcpConn) Read(b []byte) (int, error) {
	if err := c.TCPConn.SetReadDeadline(time.Now().Add(c.ReadTimeout)); err != nil {
		return 0, err
	}

	return c.TCPConn.Read(b)
}

func (c *tcpConn) Write(b []byte) (int, error) {
	c.writeLock.Lock()
	defer c.writeLock.Unlock()

	if err := c.TCPConn.SetWriteDeadline(time.Now().Add(c.WriteTimeout)); err != nil {
		return 0, err
	}

	return c.TCPConn.Write(b)
}

func (c *tcpConn) MTU() int {
	return math.MaxUint16
}
