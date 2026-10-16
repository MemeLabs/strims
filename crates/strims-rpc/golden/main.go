// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// A Go peer for crates/strims-rpc/tests/cross_go.rs: the Go RPC library on
// stdin/stdout, either serving the test Debug service or checking one as a
// client.
//
//	go run -tags golden ./crates/strims-rpc/golden server
//	go run -tags golden ./crates/strims-rpc/golden client
//
// The test service, served the same way by both sides:
//
//   - PProf answers with Name (or its length, if longer than 64 bytes) and
//     Data of the length Name spells in decimal (0 otherwise), byte i
//     being i*7;
//   - WatchMetrics streams IntervalMs items with Data [i], then closes;
//     with 0 it streams until cancelled, and a negative value fails with
//     code 3;
//   - ReadMetrics answers only when cancelled;
//   - GetConfig fails with code 7, "no config";
//   - StartMockStream answers with the number of calls cancelled so far;
//   - the other methods aren't implemented.
//
// The client exits with status 1 after printing failed checks to stderr.
package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	"go.uber.org/zap"
)

type stdio struct {
	io.Reader
	io.Writer
}

type debugService struct {
	debugv1.UnimplementedDebugService
	cancelled atomic.Int64
}

func data(n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte(i * 7)
	}
	return b
}

func (s *debugService) PProf(ctx context.Context, req *debugv1.PProfRequest) (*debugv1.PProfResponse, error) {
	name := req.Name
	if len(name) > 64 {
		name = strconv.Itoa(len(name))
	}
	n, _ := strconv.Atoi(req.Name)
	return &debugv1.PProfResponse{Name: name, Data: data(n)}, nil
}

func (s *debugService) ReadMetrics(ctx context.Context, req *debugv1.ReadMetricsRequest) (*debugv1.ReadMetricsResponse, error) {
	<-ctx.Done()
	s.cancelled.Add(1)
	return nil, ctx.Err()
}

func (s *debugService) WatchMetrics(ctx context.Context, req *debugv1.WatchMetricsRequest) (<-chan *debugv1.WatchMetricsResponse, error) {
	if req.IntervalMs < 0 {
		return nil, rpc.WrapError(errors.New("negative interval"), int32(3))
	}
	ch := make(chan *debugv1.WatchMetricsResponse)
	go func() {
		for i := 0; req.IntervalMs == 0 || i < int(req.IntervalMs); i++ {
			select {
			case ch <- &debugv1.WatchMetricsResponse{Data: []byte{byte(i)}}:
			case <-ctx.Done():
				s.cancelled.Add(1)
				return
			}
			if req.IntervalMs == 0 {
				time.Sleep(time.Millisecond)
			}
		}
		close(ch)
	}()
	return ch, nil
}

func (s *debugService) GetConfig(ctx context.Context, req *debugv1.GetConfigRequest) (*debugv1.GetConfigResponse, error) {
	return nil, rpc.WrapError(errors.New("no config"), int32(7))
}

func (s *debugService) StartMockStream(ctx context.Context, req *debugv1.StartMockStreamRequest) (*debugv1.StartMockStreamResponse, error) {
	return &debugv1.StartMockStreamResponse{Id: uint64(s.cancelled.Load())}, nil
}

func serve(logger *zap.Logger) error {
	server := rpc.NewServer(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: stdio{os.Stdin, os.Stdout},
	})
	debugv1.RegisterDebugService(server, &debugService{})
	err := server.Listen(context.Background())
	if errors.Is(err, io.EOF) {
		return nil
	}
	return err
}

type checker struct {
	failed bool
}

func (c *checker) check(name string, err error) {
	if err != nil {
		c.failed = true
		fmt.Fprintf(os.Stderr, "%s: %v\n", name, err)
	}
}

func expectError(err error, code int32, message string) error {
	if err == nil {
		return errors.New("no error")
	}
	if rpc.ErrorCode(err) != code || err.Error() != message {
		return fmt.Errorf("got %q (code %d), want %q (code %d)", err, rpc.ErrorCode(err), message, code)
	}
	return nil
}

// watch runs a WatchMetrics call, handing items to f until it returns
// false, and returns the call's error.
func watch(ctx context.Context, c *debugv1.DebugClient, interval int32, f func(*debugv1.WatchMetricsResponse) bool) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	ch := make(chan *debugv1.WatchMetricsResponse)
	done := make(chan error, 1)
	go func() {
		done <- c.WatchMetrics(ctx, &debugv1.WatchMetricsRequest{IntervalMs: interval}, ch)
	}()
	for item := range ch {
		if !f(item) {
			cancel()
			for range ch {
			}
			break
		}
	}
	return <-done
}

// waitCancelled polls the server's count of cancelled calls until it
// reaches n.
func waitCancelled(ctx context.Context, c *debugv1.DebugClient, n uint64) error {
	for i := 0; i < 500; i++ {
		res := &debugv1.StartMockStreamResponse{}
		if err := c.StartMockStream(ctx, &debugv1.StartMockStreamRequest{}, res); err != nil {
			return err
		}
		if res.Id >= n {
			return nil
		}
		time.Sleep(10 * time.Millisecond)
	}
	return fmt.Errorf("the server saw fewer than %d cancels", n)
}

func runClient(logger *zap.Logger) error {
	client, err := rpc.NewClient(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: stdio{os.Stdin, os.Stdout},
	})
	if err != nil {
		return err
	}
	defer client.Close()
	c := debugv1.NewDebugClient(client)
	ctx := context.Background()
	var k checker

	for _, n := range []int{0, 1000, 500_000} {
		k.check(fmt.Sprintf("pprof %d", n), func() error {
			res := &debugv1.PProfResponse{}
			if err := c.PProf(ctx, &debugv1.PProfRequest{Name: strconv.Itoa(n)}, res); err != nil {
				return err
			}
			if res.Name != strconv.Itoa(n) || string(res.Data) != string(data(n)) {
				return fmt.Errorf("got name %q and %d bytes", res.Name, len(res.Data))
			}
			return nil
		}())
	}

	k.check("watch 5", func() error {
		var got []byte
		err := watch(ctx, c, 5, func(r *debugv1.WatchMetricsResponse) bool {
			got = append(got, r.Data...)
			return true
		})
		if err != nil {
			return err
		}
		if string(got) != "\x00\x01\x02\x03\x04" {
			return fmt.Errorf("got items %v", got)
		}
		return nil
	}())

	k.check("watch until cancelled", func() error {
		n := 0
		err := watch(ctx, c, 0, func(r *debugv1.WatchMetricsResponse) bool {
			n++
			return n < 3
		})
		if !errors.Is(err, context.Canceled) {
			return fmt.Errorf("call ended with %v", err)
		}
		return waitCancelled(ctx, c, 1)
	}())

	k.check("unary deadline", func() error {
		tctx, cancel := context.WithTimeout(ctx, 200*time.Millisecond)
		defer cancel()
		err := c.ReadMetrics(tctx, &debugv1.ReadMetricsRequest{}, &debugv1.ReadMetricsResponse{})
		if !errors.Is(err, context.DeadlineExceeded) {
			return fmt.Errorf("call ended with %v", err)
		}
		return waitCancelled(ctx, c, 2)
	}())

	k.check("watch negative", expectError(
		watch(ctx, c, -1, func(*debugv1.WatchMetricsResponse) bool { return true }),
		3, "negative interval",
	))
	k.check("get config", expectError(
		c.GetConfig(ctx, &debugv1.GetConfigRequest{}, &debugv1.GetConfigResponse{}),
		7, "no config",
	))
	k.check("set config", expectError(
		c.SetConfig(ctx, &debugv1.SetConfigRequest{}, &debugv1.SetConfigResponse{}),
		0, "not implemented",
	))
	k.check("unknown method", expectError(
		client.CallUnary(ctx, "strims.debug.v1.Debug.Nope", &debugv1.PProfRequest{}, &debugv1.PProfResponse{}),
		0, "method not found: strims.debug.v1.Debug.Nope",
	))

	if k.failed {
		return errors.New("checks failed")
	}
	return nil
}

func main() {
	logger := zap.NewNop()
	var err error
	switch mode := os.Args[len(os.Args)-1]; mode {
	case "server":
		err = serve(logger)
	case "client":
		err = runClient(logger)
	default:
		err = fmt.Errorf("unknown mode %q", mode)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
