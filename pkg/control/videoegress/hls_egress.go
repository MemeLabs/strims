// +build !js

package videoegress

import (
	"net"
	"net/http"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/hls"
	"go.uber.org/zap"
)

const defaultEgressAddr = "127.0.0.1:0"

func newHLSEgress(logger *zap.Logger) (*hlsEgress, error) {
	srv := hls.NewService()

	return &hlsEgress{
		logger: logger,
		svc:    srv,
		srv: http.Server{
			Handler: srv.Handler(),
		},
	}, nil
}

type hlsEgress struct {
	logger *zap.Logger
	svc    *hls.Service
	srv    http.Server
	lock   sync.Mutex
	lis    net.Listener
	nextID uint64
}

func (e *hlsEgress) Serve() error {
	return e.ServeWithAddr(defaultEgressAddr)
}

func (e *hlsEgress) ServeWithAddr(addr string) error {
	lis, err := e.listen(addr)
	if err != nil {
		return err
	}

	return e.srv.Serve(lis)
}

func (e *hlsEgress) listen(addr string) (net.Listener, error) {
	e.lock.Lock()
	defer e.lock.Unlock()

	lis, err := net.Listen("tcp", addr)
	if err != nil {
		return nil, err
	}
	e.lis = lis

	return lis, nil
}

func (e *hlsEgress) Close() error {
	e.lock.Lock()
	defer e.lock.Unlock()

	if err := e.srv.Close(); err != nil {
		return err
	}

	if err := e.lis.Close(); err != nil {
		return err
	}
	e.lis = nil

	return nil
}

func (e *hlsEgress) Add() {
	// stream := hls.NewStream(hls.DefaultStreamOptions)
	// go func() {
	// 	if err := e.svc.SendStream(context.Background(), stream); err != nil {
	// 		e.logger.Debug("sending stream to hls egress failed", zap.Error(err))
	// 	}
	// }()

	// id := atomic.AddUint64(&e.nextID, 1)

	// e.svc.InsertChannel(&hls.Channel{
	// 	Name:   strconv.FormatUint(id, 10),
	// 	Stream: stream,
	// })

	// fmt.Sprintf("http://%s/hls/%d/index.m3u8", e.lis.Addr().String(), id)
}

func (e *hlsEgress) Remove(id string) {
	e.svc.RemoveChannel(&hls.Channel{Name: id})
}
