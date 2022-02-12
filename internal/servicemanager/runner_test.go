package servicemanager

import (
	"context"
	"crypto/rand"
	"errors"
	"testing"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv/kvtest"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestRunner(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.NoError(t, err)

	var key [dao.KeySize]byte
	_, err = rand.Read(key[:])
	assert.NoError(t, err)
	skey, err := dao.NewStorageKeyFromBytes(key[:], nil)
	assert.NoError(t, err)
	store := dao.NewProfileStore(0, skey, kvtest.NewMemStore(), nil)
	assert.NoError(t, store.Init())

	ctx := context.Background()

	r, _ := New[MockReader](logger, ctx, &MockService{logger, store})

	reader, close, err := r.Reader(ctx)
	assert.NoError(t, err)
	defer close()

	if reader.source == "server" {
		return
	}

	<-reader.done

	reader, close, err = r.Reader(ctx)
	assert.NoError(t, err)
	defer close()

	assert.Equal(t, "server", reader.source)
}

type MockReader struct {
	source string
	done   chan struct{}
}

type MockServer struct {
	done chan struct{}
}

func (s *MockServer) Reader(ctx context.Context) (MockReader, error) {
	return MockReader{"server", s.done}, nil
}

func (s *MockServer) Run(ctx context.Context) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	case <-s.done:
		return errors.New("closed")
	}
}

func (s *MockServer) Close(ctx context.Context) error {
	close(s.done)
	return nil
}

type MockClient struct {
	done chan struct{}
}

func (s *MockClient) Reader(ctx context.Context) (MockReader, error) {
	return MockReader{"client", s.done}, nil
}

func (s *MockClient) Run(ctx context.Context) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	case <-s.done:
		return errors.New("closed")
	}
}

func (s *MockClient) Close(ctx context.Context) error {
	close(s.done)
	return nil
}

type MockService struct {
	logger *zap.Logger
	store  *dao.ProfileStore
}

func (s *MockService) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "test")
}

func (s *MockService) Client() (Readable[MockReader], error) {
	return &MockClient{make(chan struct{})}, nil
}

func (s *MockService) Server() (Readable[MockReader], error) {
	return &MockServer{make(chan struct{})}, nil
}
