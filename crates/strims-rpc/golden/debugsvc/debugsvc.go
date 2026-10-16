// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Package debugsvc is the test Debug service that the Go peers of the Rust
// RPC tests serve (crates/strims-rpc/golden, crates/strims-node/golden/interop),
// and that the Rust tests serve the same way:
//
//   - PProf answers with Name (or its length, if longer than 64 bytes) and
//     Data of the length Name spells in decimal (0 otherwise), byte i
//     being i*7;
//   - WatchMetrics streams IntervalMs items with Data [i], then closes;
//     with 0 it streams one item per millisecond until cancelled, and a
//     negative value fails with code 3;
//   - ReadMetrics answers only when cancelled;
//   - GetConfig fails with code 7, "no config";
//   - StartMockStream answers with the number of calls cancelled so far;
//   - the other methods aren't implemented.
package debugsvc

import (
	"context"
	"errors"
	"strconv"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
)

// Data is PProf's payload of n bytes.
func Data(n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte(i * 7)
	}
	return b
}

// Service implements debugv1.DebugService as described above.
type Service struct {
	debugv1.UnimplementedDebugService
	cancelled atomic.Int64
}

func (s *Service) PProf(ctx context.Context, req *debugv1.PProfRequest) (*debugv1.PProfResponse, error) {
	name := req.Name
	if len(name) > 64 {
		name = strconv.Itoa(len(name))
	}
	n, _ := strconv.Atoi(req.Name)
	return &debugv1.PProfResponse{Name: name, Data: Data(n)}, nil
}

func (s *Service) ReadMetrics(ctx context.Context, req *debugv1.ReadMetricsRequest) (*debugv1.ReadMetricsResponse, error) {
	<-ctx.Done()
	s.cancelled.Add(1)
	return nil, ctx.Err()
}

func (s *Service) WatchMetrics(ctx context.Context, req *debugv1.WatchMetricsRequest) (<-chan *debugv1.WatchMetricsResponse, error) {
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

func (s *Service) GetConfig(ctx context.Context, req *debugv1.GetConfigRequest) (*debugv1.GetConfigResponse, error) {
	return nil, rpc.WrapError(errors.New("no config"), int32(7))
}

func (s *Service) StartMockStream(ctx context.Context, req *debugv1.StartMockStreamRequest) (*debugv1.StartMockStreamResponse, error) {
	return &debugv1.StartMockStreamResponse{Id: uint64(s.cancelled.Load())}, nil
}
