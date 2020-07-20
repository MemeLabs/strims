package rpc

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"log"
	"reflect"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"github.com/golang/protobuf/ptypes"
	"github.com/golang/protobuf/ptypes/any"
	"go.uber.org/zap"
)

const (
	callbackMethod = "_CALLBACK"
	cancelMethod   = "_CANCEL"

	anyURLPrefix = "strims.gg/"
)

type conn struct {
	callID    uint64
	callbacks sync.Map
	calls     sync.Map
	w         io.Writer
}

type callKey struct {
	c  *conn
	id uint64
}

func handleCallback(c *conn, m *pb.Call) {
	ci, ok := c.callbacks.Load(m.ParentId)
	if !ok {
		log.Println("dropped message without handler...", m)
		return
	}
	ci.(chan *pb.Call) <- m
}

func handleCancel(c *conn, m *pb.Call) {
	if cancel, ok := c.calls.Load(callKey{c, m.ParentId}); ok {
		cancel.(context.CancelFunc)()
	}
}

func readCall(r io.Reader) (*pb.Call, error) {
	b := *readBuffers.Get().(*[]byte)
	defer readBuffers.Put(&b)

	l, err := binary.ReadUvarint(bytereader.New(r))
	if err != nil {
		return nil, err
	}
	if int(l) > cap(b) {
		b = make([]byte, l)
	}
	b = b[:l]

	if _, err := io.ReadAtLeast(r, b, int(l)); err != nil {
		return nil, err
	}

	m := &pb.Call{}
	if err := proto.Unmarshal(b, m); err != nil {
		return nil, err
	}

	return m, nil
}

var readBuffers = sync.Pool{
	New: func() interface{} {
		b := make([]byte, 1024)
		return &b
	},
}

func recoverError(v interface{}) error {
	switch err := v.(type) {
	case nil:
		return nil
	case error:
		return err
	case string:
		return errors.New(err)
	default:
		return errors.New("unknown error")
	}
}

var callBuffers = sync.Pool{
	New: func() interface{} {
		return proto.NewBuffer([]byte{})
	},
}

// CallOption call option setter
type CallOption func(c *pb.Call)

func withParentID(id uint64) CallOption {
	return func(c *pb.Call) {
		c.ParentId = id
	}
}

// call invoke a remote procedure
func call(ctx context.Context, c *conn, method string, v proto.Message, opts ...CallOption) error {
	ab := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(ab)
	ab.Reset()

	if err := ab.Marshal(v); err != nil {
		return err
	}

	m := &pb.Call{
		Id:     atomic.AddUint64(&c.callID, 1),
		Method: method,
		Argument: &any.Any{
			TypeUrl: anyURLPrefix + proto.MessageName(v),
			Value:   ab.Bytes(),
		},
	}

	for _, o := range opts {
		o(m)
	}

	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	if err := b.EncodeVarint(uint64(proto.Size(m))); err != nil {
		return err
	}
	if err := b.Marshal(m); err != nil {
		return err
	}

	if _, err := c.w.Write(b.Bytes()); err != nil {
		return err
	}

	return nil
}

func newCallbackReceiver(logger *zap.Logger, c *conn) *callbackReceiver {
	return &callbackReceiver{
		logger: logger,
		conn:   c,
		res:    make(chan *pb.Call, 1),
	}
}

type callbackReceiver struct {
	logger *zap.Logger
	conn   *conn
	res    chan *pb.Call
	call   *pb.Call
}

func (r *callbackReceiver) CallOption() CallOption {
	return func(c *pb.Call) {
		r.call = c
		r.conn.callbacks.Store(c.Id, r.res)
	}
}

func (r *callbackReceiver) ReceiveUnary(ctx context.Context, v proto.Message) error {
	defer r.conn.callbacks.Delete(r.call.Id)

	select {
	case <-ctx.Done():
		if err := call(context.Background(), r.conn, cancelMethod, &pb.Cancel{}, withParentID(r.call.Id)); err != nil {
			return err
		}
		return ctx.Err()
	case res := <-r.res:
		return unmarshalAny(res.Argument, v)
	}
}

var typeOfProtoMessage = reflect.TypeOf((*proto.Message)(nil)).Elem()

func (r *callbackReceiver) ReceiveStream(ctx context.Context, ch interface{}) {
	chv := reflect.ValueOf(ch)
	if chv.Kind() != reflect.Chan || !chv.Type().Elem().Implements(typeOfProtoMessage) {
		panic("ch must be a chan of a type that implements proto.Message")
	}

	defer func() {
		r.conn.callbacks.Delete(r.call.Id)
		chv.Close()
	}()

	for {
		select {
		case <-ctx.Done():
			if err := call(context.Background(), r.conn, cancelMethod, &pb.Cancel{}, withParentID(r.call.Id)); err != nil {
				r.logger.Debug("call failed", zap.Error(err))
			}
			return
		case res := <-r.res:
			v := reflect.New(chv.Type().Elem().Elem())
			if err := unmarshalAny(res.Argument, v.Interface().(proto.Message)); err != nil {
				return
			}
			chv.Send(v)
		}
	}
}

var typeOfError = reflect.TypeOf(&pb.Error{})
var typeOfClose = reflect.TypeOf(&pb.Close{})
var errClose = errors.New("response closed")
var errInvalidType = errors.New("invaild type")

func newAnyMessage(a *any.Any) (proto.Message, error) {
	n, err := ptypes.AnyMessageName(a)
	if err != nil {
		return nil, err
	}
	k := proto.MessageType(n)
	if k == nil {
		return nil, errInvalidType
	}
	return reflect.New(k.Elem()).Interface().(proto.Message), nil
}

func unmarshalAny(a *any.Any, v proto.Message) error {
	n, err := ptypes.AnyMessageName(a)
	if err != nil {
		return err
	}

	at := proto.MessageType(n)
	vt := reflect.TypeOf(v)
	switch at {
	case vt:
		return ptypes.UnmarshalAny(a, v)
	case typeOfClose:
		return errClose
	case typeOfError:
		ev := &pb.Error{}
		if err := ptypes.UnmarshalAny(a, ev); err != nil {
			return err
		}
		return errors.New(ev.Message)
	default:
		return fmt.Errorf("Using %s as type %s", at, vt)
	}
}
