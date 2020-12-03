package rpc

import (
	"context"
	"encoding/binary"
	"errors"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"go.uber.org/zap"
)

const defaultMaxMessageBytes = 512 * 1024

// ErrMessageTooLarge emitted when received message exceeds configured limit
var ErrMessageTooLarge = errors.New("received message too large")

// RWDialer ...
type RWDialer struct {
	Logger          *zap.Logger
	ReadWriter      io.ReadWriter
	MaxMessageBytes int
}

// Dial ...
func (d *RWDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	maxMessageBytes := d.MaxMessageBytes
	if maxMessageBytes == 0 {
		maxMessageBytes = defaultMaxMessageBytes
	}

	return &RWTransport{
		ctx:             ctx,
		logger:          d.Logger,
		rw:              d.ReadWriter,
		maxMessageBytes: maxMessageBytes,
		dispatcher:      dispatcher,
	}, nil
}

// RWTransport ...
type RWTransport struct {
	ctx             context.Context
	logger          *zap.Logger
	rw              io.ReadWriter
	maxMessageBytes int
	callsIn         sync.Map
	callsOut        sync.Map
	dispatcher      Dispatcher
}

// Listen reads incoming calls
func (t *RWTransport) Listen() error {
	var b []byte

	for {
		l, err := binary.ReadUvarint(bytereader.New(t.rw))
		if err != nil {
			return err
		}
		if int(l) > t.maxMessageBytes {
			return ErrMessageTooLarge
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

		parentCallAccessor := &rwParentCallAccessor{
			id:       req.ParentId,
			callsIn:  &t.callsIn,
			callsOut: &t.callsOut,
		}
		call := NewCallIn(t.ctx, req, parentCallAccessor, t.send)

		t.callsIn.Store(req.Id, call)
		go func() {
			t.dispatcher.Dispatch(call)
			t.callsIn.Delete(req.Id)
		}()

		if err := t.ctx.Err(); err != nil {
			return err
		}
	}
}

func (t *RWTransport) send(ctx context.Context, call *pb.Call) error {
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
	t.callsOut.Store(call.ID(), call)
	defer t.callsOut.Delete(call.ID())

	if err := call.SendRequest(t.send); err != nil {
		return err
	}

	return fn()
}

type rwParentCallAccessor struct {
	id       uint64
	callsIn  *sync.Map
	callsOut *sync.Map
}

func (a *rwParentCallAccessor) ParentCallIn() *CallIn {
	if p, ok := a.callsIn.Load(a.id); ok {
		return p.(*CallIn)
	}
	return nil
}

func (a *rwParentCallAccessor) ParentCallOut() *CallOut {
	if p, ok := a.callsOut.Load(a.id); ok {
		return p.(*CallOut)
	}
	return nil
}
