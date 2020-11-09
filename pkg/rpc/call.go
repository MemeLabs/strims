package rpc

import (
	"context"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
)

// ResponseType ...
type ResponseType int

func (t ResponseType) String() string {
	return responseTypeName[t]
}

// ResponseTypes ...
const (
	ResponseTypeNone = iota
	ResponseTypeUndefined
	ResponseTypeError
	ResponseTypeStream
	ResponseTypeValue
)

var responseTypeName = map[ResponseType]string{
	ResponseTypeNone:      "NONE",
	ResponseTypeUndefined: "UNDEFINED",
	ResponseTypeError:     "ERROR",
	ResponseTypeStream:    "STREAM",
	ResponseTypeValue:     "VALUE",
}

// Dispatcher ...
type Dispatcher interface {
	Dispatch(*CallIn)
}

// Call ...
type Call interface {
	ID() uint64
}

// NewCallBase ...
func NewCallBase(ctx context.Context) CallBase {
	ctx, cancel := context.WithCancel(ctx)

	return CallBase{
		ctx:    ctx,
		cancel: cancel,
		res:    make(chan proto.Message),
	}
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
func NewCallIn(ctx context.Context, req *pb.Call, parentCallAcessor ParentCallAccessor) *CallIn {
	return &CallIn{
		CallBase:           NewCallBase(ctx),
		req:                req,
		ParentCallAccessor: parentCallAcessor,
	}
}

// CallIn ...
type CallIn struct {
	CallBase
	ParentCallAccessor
	req          *pb.Call
	responseType ResponseType
}

// ID ...
func (c *CallIn) ID() uint64 {
	return c.req.Id
}

// Method ...
func (c *CallIn) Method() string {
	return c.req.Method
}

// ResponseType ...
func (c *CallIn) ResponseType() ResponseType {
	return c.responseType
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

func (c *CallIn) returnNone() {
	close(c.res)
}

func (c *CallIn) returnUndefined() {
	c.responseType = ResponseTypeUndefined
	c.res <- &pb.Undefined{}
	close(c.res)
}

func (c *CallIn) returnError(err error) {
	c.responseType = ResponseTypeError
	c.res <- &pb.Error{Message: err.Error()}
	close(c.res)
}

func (c *CallIn) returnValue(v proto.Message) {
	c.responseType = ResponseTypeValue
	c.res <- v
	close(c.res)
}

func (c *CallIn) returnStream(v interface{}) {
	c.responseType = ResponseTypeStream
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

var emptyCallOut = &CallOut{}

// NewCallOut ...
func NewCallOut(ctx context.Context, method string, arg proto.Message) (*CallOut, error) {
	return NewCallOutWithParent(ctx, method, arg, emptyCallOut)
}

// NewCallOutWithParent ...
func NewCallOutWithParent(ctx context.Context, method string, arg proto.Message, parent Call) (*CallOut, error) {
	id, err := dao.GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	return &CallOut{
		CallBase: NewCallBase(ctx),
		id:       id,
		parentID: parent.ID(),
		method:   method,
		arg:      arg,
		errs:     make(chan error),
	}, nil
}

// CallOut ...
type CallOut struct {
	CallBase
	id       uint64
	parentID uint64
	method   string
	arg      proto.Message
	errs     chan error
}

// ID ...
func (c *CallOut) ID() uint64 {
	return c.id
}

// Method ...
func (c *CallOut) Method() string {
	return c.method
}

// SendRequest ...
func (c *CallOut) SendRequest(fn SendFunc) error {
	return send(c.ctx, c.id, c.parentID, c.method, c.arg, fn)
}

// AssignResponse ...
func (c *CallOut) AssignResponse(res *CallIn) {
	select {
	case r := <-c.res:
		c.errs <- unmarshalAny(res.req.Argument, r)
	case <-c.ctx.Done():
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
		err := c.ReadResponse(v.Interface().(proto.Message))
		if err == ErrClose {
			return nil
		} else if err != nil {
			return err
		}
		ch.Send(v)
	}
}
