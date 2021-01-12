package rpc

import (
	"context"
	"testing"
	"time"

	rpcv1test "github.com/MemeLabs/go-ppspp/pkg/apis/rpc/v1/test"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/ppspptest"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

type testServer struct{}

func (t *testServer) CallUnary(ctx context.Context, req *rpcv1test.RPCCallUnaryRequest) (*rpcv1test.RPCCallUnaryResponse, error) {
	return &rpcv1test.RPCCallUnaryResponse{Id: req.Id}, nil
}

func (t *testServer) CallStream(ctx context.Context, req *rpcv1test.RPCCallStreamRequest) (<-chan *rpcv1test.RPCCallStreamResponse, error) {
	ch := make(chan *rpcv1test.RPCCallStreamResponse)

	go func() {
		for i := 0; i < int(req.Count); i++ {
			ch <- &rpcv1test.RPCCallStreamResponse{Id: req.Id}
			time.Sleep(time.Millisecond)
		}
		close(ch)
	}()

	return ch, nil
}

func newTestClientServerPair(logger *zap.Logger) (*rpcv1test.RPCTestClient, *rpc.Server, error) {
	a, b := ppspptest.NewUnbufferedConnPair()

	server := rpc.NewServer(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: a,
	})
	rpcv1test.RegisterRPCTestService(server, &testServer{})
	go server.Listen(context.Background())

	client, err := rpc.NewClient(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: b,
	})
	return rpcv1test.NewRPCTestClient(client), server, err
}

func TestUnaryE2E(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	client, _, err := newTestClientServerPair(logger)
	assert.Nil(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	req := &rpcv1test.RPCCallUnaryRequest{Id: 1}
	res := &rpcv1test.RPCCallUnaryResponse{}
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

	req := &rpcv1test.RPCCallStreamRequest{Id: 1, Count: 3}
	res := make(chan *rpcv1test.RPCCallStreamResponse)
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
		req := &rpcv1test.RPCCallUnaryRequest{Id: 1}
		res := &rpcv1test.RPCCallUnaryResponse{}
		client.CallUnary(ctx, req, res)
	}
}
