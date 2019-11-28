package service

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log"
	"reflect"
	"strings"
	"sync"
	"sync/atomic"

	"github.com/golang/protobuf/proto"
	"github.com/golang/protobuf/ptypes"
	"github.com/golang/protobuf/ptypes/any"
)

const headerLen = 4
const callbackMethod = "callback"

// NewRPCHost ...
func NewRPCHost(w io.Writer, r io.Reader, service interface{}) *RPCHost {
	return &RPCHost{
		w:       w,
		r:       r,
		service: service,
	}
}

// RPCHost ...
type RPCHost struct {
	w         io.Writer
	r         io.Reader
	callID    uint64
	callbacks sync.Map
	service   interface{}
}

// Run starts reading incoming calls
func (c *RPCHost) Run(ctx context.Context) error {
	for {
		m, err := readCall(c.r)
		if err != nil {
			return err
		}

		if m.ParentId != 0 {
			go c.handleCallback(ctx, m)
		} else {
			go c.handleCall(ctx, m)
		}

		if err := ctx.Err(); err != nil {
			return err
		}
	}
}

var readBuffers = sync.Pool{
	New: func() interface{} {
		return make([]byte, 1024)
	},
}

func readCall(r io.Reader) (*Call, error) {
	b := readBuffers.Get().([]byte)
	defer readBuffers.Put(b)

	if _, err := io.ReadAtLeast(r, b[:headerLen], headerLen); err != nil {
		return nil, err
	}

	l, err := proto.NewBuffer(b).DecodeFixed32()
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

	m := &Call{}
	if err := proto.Unmarshal(b, m); err != nil {
		return nil, err
	}

	return m, nil
}

func (c *RPCHost) handleCallback(ctx context.Context, m *Call) {
	ci, ok := c.callbacks.Load(m.ParentId)
	if !ok {
		log.Println("dropped message without handler...", m)
		return
	}
	ci.(chan *Call) <- m
}

func recoverError(v interface{}) error {
	switch err := v.(type) {
	case error:
		return err
	case string:
		return errors.New(err)
	default:
		return errors.New("unknown error")
	}
}

func (c *RPCHost) handleCall(ctx context.Context, m *Call) {
	defer func() {
		if err := recover(); err != nil {
			e := &Error{Message: recoverError(err).Error()}
			c.Call(ctx, callbackMethod, e, withParentID(m.GetId()))
		}
	}()

	arg, err := newAnyMessage(m.GetArgument())
	if err != nil {
		return
	}
	if err := unmarshalAny(m.GetArgument(), arg); err != nil {
		return
	}

	method := reflect.ValueOf(c.service).MethodByName(strings.Title(m.Method))
	if !method.IsValid() {
		e := &Error{Message: fmt.Sprintf("undefined method: %s", m.Method)}
		c.Call(ctx, callbackMethod, e, withParentID(m.GetId()))
		return
	}

	rs := method.Call([]reflect.Value{reflect.ValueOf(ctx), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		c.Call(ctx, callbackMethod, &Undefined{}, withParentID(m.GetId()))
		return
	}

	if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		c.Call(ctx, callbackMethod, &Error{Message: err.Error()}, withParentID(m.GetId()))
		return
	}

	if r := rs[0]; r.Kind() == reflect.Chan {
		for {
			a, ok := r.Recv()
			if !ok {
				c.Call(ctx, callbackMethod, &Close{}, withParentID(m.GetId()))
				return
			}
			c.Call(ctx, callbackMethod, a.Interface().(proto.Message), withParentID(m.GetId()))
		}
	}

	if a, ok := rs[0].Interface().(proto.Message); ok {
		c.Call(ctx, callbackMethod, a, withParentID(m.GetId()))
	}
}

var callBuffers = sync.Pool{
	New: func() interface{} {
		return proto.NewBuffer([]byte{})
	},
}

// CallOption call option setter
type CallOption func(c *Call)

func withParentID(id uint64) CallOption {
	return func(c *Call) {
		c.ParentId = id
	}
}

// Call invoke a remote procedure
func (c *RPCHost) Call(ctx context.Context, method string, v proto.Message, opts ...CallOption) (*Call, error) {
	any, err := ptypes.MarshalAny(v)
	if err != nil {
		return nil, err
	}

	m := &Call{
		Id:       atomic.AddUint64(&c.callID, 1),
		Method:   method,
		Argument: any,
	}
	for _, o := range opts {
		o(m)
	}

	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	b.EncodeFixed32(uint64(proto.Size(m)))
	if err := b.Marshal(m); err != nil {
		return nil, err
	}

	if _, err := c.w.Write(b.Bytes()); err != nil {
		return nil, err
	}

	return m, nil
}

// ExpectOne blocks until the call returns and decodes the response into v
func (c *RPCHost) ExpectOne(ctx context.Context, m *Call, v proto.Message) error {
	ch := make(chan *Call, 1)
	c.callbacks.Store(m.Id, ch)
	defer c.callbacks.Delete(m.Id)

	select {
	case <-ctx.Done():
		return ctx.Err()
	case res := <-ch:
		return unmarshalAny(res.GetArgument(), v)
	}
}

var typeOfProtoMessage = reflect.TypeOf((*proto.Message)(nil)).Elem()

// ExpectMany passes callback values to ch. ch should be a chan of some type
// that implements proto.Message.
func (c *RPCHost) ExpectMany(ctx context.Context, m *Call, ch interface{}) error {
	chv := reflect.ValueOf(ch)
	if chv.Kind() != reflect.Chan || !chv.Type().Implements(typeOfProtoMessage) {
		panic("ch must be a chan of a type that implements proto.Message")
	}

	cch := make(chan *Call, 1)
	c.callbacks.Store(m.Id, cch)

	go func() {
		defer func() {
			c.callbacks.Delete(m.Id)
			chv.Close()
		}()

		for {
			select {
			case <-ctx.Done():
				return
			case res := <-cch:
				v := reflect.New(chv.Type().Elem())
				if err := unmarshalAny(res.GetArgument(), v.Interface().(proto.Message)); err != nil {
					return
				}
				chv.Send(v)
			}
		}
	}()

	return nil
}

var typeOfError = reflect.TypeOf(&Error{})
var typeOfClose = reflect.TypeOf(&Close{})
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
		ev := &Error{}
		if err := ptypes.UnmarshalAny(a, ev); err != nil {
			return err
		}
		return errors.New(ev.Message)
	default:
		return fmt.Errorf("Using %s as type %s", at, vt)
	}
}
