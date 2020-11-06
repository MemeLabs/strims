package rpc

import (
	"context"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
)

// Dispatcher ...
type Dispatcher interface {
	Dispatch(*CallIn)
}

// Call ...
type Call interface {
	ID() uint64
}

// CallBase ...
type CallBase struct {
	ctx    context.Context
	cancel context.CancelFunc
	res    chan proto.Message
}

// Context ...
func (c *CallBase) Context() context.Context {
	return c.ctx
}

// Cancel ...
func (c *CallBase) Cancel() {
	c.cancel()
}

// NewCallIn ...
func NewCallIn(ctx context.Context, req *pb.Call, parent Call) *CallIn {
	ctx, cancel := context.WithCancel(ctx)

	return &CallIn{
		CallBase: CallBase{
			ctx:    ctx,
			cancel: cancel,
			res:    make(chan proto.Message),
		},
		req:    req,
		parent: parent,
	}
}

// CallIn ...
type CallIn struct {
	CallBase
	req    *pb.Call
	parent Call
}

// ID ...
func (c *CallIn) ID() uint64 {
	return c.req.Id
}

// Method ...
func (c *CallIn) Method() string {
	return c.req.Method
}

// Parent ...
func (c *CallIn) Parent() Call {
	return c.parent
}

// Argument ...
func (c *CallIn) Argument() (interface{}, error) {
	arg, err := newAnyMessage(c.req.Argument)
	if err != nil {
		return nil, err
	}
	if err := unmarshalAny(c.req.Argument, arg); err != nil {
		return nil, err
	}
	return arg, nil
}

// SendResponse ...
func (c *CallIn) SendResponse(fn SendFunc) error {
	for res := range c.res {
		id, err := dao.GenerateSnowflake()
		if err != nil {
			return err
		}

		if err := send(c.ctx, id, c.req.Id, callbackMethod, res, fn); err != nil {
			return err
		}
	}
	return nil
}

func (c *CallIn) returnUndefined() {
	c.res <- &pb.Undefined{}
	close(c.res)
}

func (c *CallIn) returnError(err error) {
	c.res <- &pb.Error{Message: err.Error()}
	close(c.res)
}

func (c *CallIn) returnValue(v proto.Message) {
	c.res <- v
	close(c.res)
}

func (c *CallIn) returnStream(v interface{}) {
	defer close(c.res)

	cases := []reflect.SelectCase{
		{
			Dir:  reflect.SelectRecv,
			Chan: reflect.ValueOf(v),
		},
		{
			Dir:  reflect.SelectRecv,
			Chan: reflect.ValueOf(c.ctx.Done()),
		},
	}

	for {
		_, v, ok := reflect.Select(cases)

		if c.ctx.Err() != nil {
			return
		}

		if !ok {
			c.res <- &pb.Close{}
			return
		}
		c.res <- v.Interface().(proto.Message)
	}
}

// NewCallOut ...
func NewCallOut(ctx context.Context, method string, arg proto.Message) (*CallOut, error) {
	id, err := dao.GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(ctx)

	return &CallOut{
		CallBase: CallBase{
			ctx:    ctx,
			cancel: cancel,
			res:    make(chan proto.Message),
		},
		id:     id,
		method: method,
		arg:    arg,
		errs:   make(chan error),
	}, nil
}

// CallOut ...
type CallOut struct {
	CallBase
	id     uint64
	method string
	arg    proto.Message
	errs   chan error
}

// ID ...
func (c *CallOut) ID() uint64 {
	return c.id
}

// SendRequest ...
func (c *CallOut) SendRequest(fn SendFunc) error {
	return send(c.ctx, c.id, 0, c.method, c.arg, fn)
}

// AssignResponse ...
func (c *CallOut) AssignResponse(res *CallIn) {
	for {
		r := <-c.res
		c.errs <- unmarshalAny(res.req.Argument, r)
	}
}

// ReadResponse ...
func (c *CallOut) ReadResponse(out proto.Message) error {
	select {
	case c.res <- out:
		if err := <-c.errs; err != nil {
			return err
		}
	case <-c.ctx.Done():
		// if err := call(context.Background(), r.conn, cancelMethod, &pb.Cancel{}, withParentID(r.call.Id)); err != nil {
		// 	r.logger.Debug("call failed", zap.Error(err))
		// }
		return c.ctx.Err()
	}
	return nil
}

var typeOfProtoMessage = reflect.TypeOf((*proto.Message)(nil)).Elem()

// ReadResponseStream ...
func (c *CallOut) ReadResponseStream(res interface{}) error {
	ch := reflect.ValueOf(res)
	if ch.Kind() != reflect.Chan || !ch.Type().Elem().Implements(typeOfProtoMessage) {
		panic("res must be a chan of a type that implements proto.Message")
	}

	defer ch.Close()

	for {
		v := reflect.New(ch.Type().Elem().Elem())
		if err := c.ReadResponse(v.Interface().(proto.Message)); err != nil {
			return err
		}
		ch.Send(v)
	}
}
