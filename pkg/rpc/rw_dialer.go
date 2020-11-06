package rpc

import (
	"context"
	"encoding/binary"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"go.uber.org/zap"
)

// RWDialer ...
type RWDialer struct {
	Logger     *zap.Logger
	ReadWriter io.ReadWriter
}

// Dial ...
func (d *RWDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	t := &RWTransport{
		ctx:        ctx,
		logger:     d.Logger,
		rw:         d.ReadWriter,
		dispatcher: dispatcher,
	}

	go t.Listen()

	return t, nil
}

// RWTransport ...
type RWTransport struct {
	ctx        context.Context
	logger     *zap.Logger
	rw         io.ReadWriter
	calls      sync.Map
	dispatcher Dispatcher
}

// Listen starts reading incoming calls
func (t *RWTransport) Listen() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	var b []byte

	for {
		l, err := binary.ReadUvarint(bytereader.New(t.rw))
		if err != nil {
			return err
		}
		if int(l) > cap(b) {
			b = make([]byte, l)
		}
		b = b[:l]

		if _, err := io.ReadAtLeast(t.rw, b, int(l)); err != nil {
			return err
		}

		req := &pb.Call{}
		if err := proto.Unmarshal(b, req); err != nil {
			continue
		}

		var parent *CallOut
		if p, ok := t.calls.Load(req.ParentId); ok {
			parent = p.(*CallOut)
		}
		call := NewCallIn(ctx, req, parent)
		t.calls.Store(req.Id, call)

		go func() {
			go t.dispatcher.Dispatch(call)
			call.SendResponse(t.call)
			t.calls.Delete(req.Id)
		}()
	}
}

func (t *RWTransport) call(ctx context.Context, call *pb.Call) error {
	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	if err := b.EncodeVarint(uint64(proto.Size(call))); err != nil {
		return err
	}
	if err := b.Marshal(call); err != nil {
		return err
	}

	if _, err := t.rw.Write(b.Bytes()); err != nil {
		return err
	}

	return nil
}

// Call ...
func (t *RWTransport) Call(call *CallOut, fn ResponseFunc) error {
	t.calls.Store(call.ID(), call)
	defer t.calls.Delete(call.ID())

	if err := call.SendRequest(t.call); err != nil {
		return err
	}

	return fn()
}
