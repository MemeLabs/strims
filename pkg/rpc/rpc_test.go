package rpc

import (
	"context"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

type testServer struct{}

func (t *testServer) CallUnary(ctx context.Context, req *pb.RPCCallUnaryRequest) (*pb.RPCCallUnaryResponse, error) {
	return &pb.RPCCallUnaryResponse{Id: req.Id}, nil
}

func (t *testServer) CallStream(ctx context.Context, req *pb.RPCCallStreamRequest) (<-chan *pb.RPCCallStreamResponse, error) {
	ch := make(chan *pb.RPCCallStreamResponse)

	go func() {
		for i := 0; i < int(req.Count); i++ {
			ch <- &pb.RPCCallStreamResponse{Id: req.Id}
			time.Sleep(time.Millisecond)
		}
		close(ch)
	}()

	return ch, nil
}

func newTestClientServerPair(logger *zap.Logger) (*api.RPCTestClient, *Server, error) {
	a, b := ppspptest.NewUnbufferedConnPair()

	server := NewServer(logger, &RWDialer{
		Logger:     logger,
		ReadWriter: a,
	})
	api.RegisterRPCTestService(server, &testServer{})
	go server.Listen(context.Background())

	client, err := NewClient(logger, &RWDialer{
		Logger:     logger,
		ReadWriter: b,
	})
	return api.NewRPCTestClient(client), server, err
}

func TestUnaryE2E(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	client, _, err := newTestClientServerPair(logger)
	assert.Nil(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	req := &pb.RPCCallUnaryRequest{Id: 1}
	res := &pb.RPCCallUnaryResponse{}
	err = client.CallUnary(ctx, req, res)
	assert.Nil(t, err)
	assert.Equal(t, req.Id, res.Id, "expected response to contain req id")
}

func TestStreamingE2E(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	client, _, err := newTestClientServerPair(logger)
	assert.Nil(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	done := make(chan struct{})

	req := &pb.RPCCallStreamRequest{Id: 1, Count: 3}
	res := make(chan *pb.RPCCallStreamResponse)
	go func() {
		err = client.CallStream(ctx, req, res)
		assert.Nil(t, err)
		close(done)
	}()

	var n uint64
	for res := range res {
		n++
		assert.Equal(t, req.Id, res.Id, "expected response to contain req id")
	}
	assert.Equal(t, req.Count, n, "expected response message count mismatch")

	<-done
}

func BenchmarkUnaryE2E(b *testing.B) {
	cfg := zap.NewDevelopmentConfig()
	cfg.Level.SetLevel(zap.ErrorLevel)
	logger, err := cfg.Build()
	assert.Nil(b, err)

	client, _, err := newTestClientServerPair(logger)
	assert.Nil(b, err)

	for i := 0; i < b.N; i++ {
		ctx := context.Background()
		req := &pb.RPCCallUnaryRequest{Id: 1}
		res := &pb.RPCCallUnaryResponse{}
		client.CallUnary(ctx, req, res)
	}
}
