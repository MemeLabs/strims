package rpc

import (
	"context"
	"fmt"
	"io"
	"reflect"
	"runtime/debug"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
)

// NewHost ...
func NewHost(service interface{}) *Host {
	return &Host{
		service: service,
	}
}

// Host ...
type Host struct {
	service interface{}
}

// Handle starts reading incoming calls
func (h *Host) Handle(ctx context.Context, w io.Writer, r io.Reader) error {
	ctx, cancel := context.WithCancel(contextWithSession(ctx, newSession()))
	defer cancel()

	c := &conn{w: w}

	for {
		m, err := readCall(r)
		if err != nil {
			return err
		}

		switch m.Method {
		case callbackMethod:
			go handleCallback(c, m)
		case cancelMethod:
			go handleCancel(c, m)
		default:
			go h.handleCall(ctx, c, m)
		}

		if err := ctx.Err(); err != nil {
			return err
		}
	}
}

func (h *Host) handleCall(ctx context.Context, c *conn, m *pb.Call) {
	ctx, cancel := context.WithCancel(ctx)
	k := callKey{c, m.Id}
	c.calls.Store(k, cancel)

	defer func() {
		c.calls.Delete(k)

		if err := recover(); err != nil {
			fmt.Printf("panic: %s\n\n%s", err, string(debug.Stack()))

			e := &pb.Error{Message: recoverError(err).Error()}
			call(ctx, c, callbackMethod, e, withParentID(m.Id))
		}
	}()

	arg, err := newAnyMessage(m.Argument)
	if err != nil {
		return
	}
	if err := unmarshalAny(m.Argument, arg); err != nil {
		return
	}

	method := reflect.ValueOf(h.service).MethodByName(strings.Title(m.Method))
	if !method.IsValid() {
		e := &pb.Error{Message: fmt.Sprintf("undefined method: %s", m.Method)}
		call(ctx, c, callbackMethod, e, withParentID(m.Id))
		return
	}

	rs := method.Call([]reflect.Value{reflect.ValueOf(ctx), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		call(ctx, c, callbackMethod, &pb.Undefined{}, withParentID(m.Id))
		return
	}

	if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		call(ctx, c, callbackMethod, &pb.Error{Message: err.Error()}, withParentID(m.Id))
		return
	}

	if r := rs[0]; r.Kind() == reflect.Chan {
		cases := []reflect.SelectCase{
			{
				Dir:  reflect.SelectRecv,
				Chan: r,
			},
			{
				Dir:  reflect.SelectRecv,
				Chan: reflect.ValueOf(ctx.Done()),
			},
		}

		for {
			_, v, ok := reflect.Select(cases)

			if ctx.Err() != nil {
				return
			}

			if !ok {
				call(ctx, c, callbackMethod, &pb.Close{}, withParentID(m.Id))
				return
			}
			call(ctx, c, callbackMethod, v.Interface().(proto.Message), withParentID(m.Id))
		}
	}

	if a, ok := rs[0].Interface().(proto.Message); ok {
		call(ctx, c, callbackMethod, a, withParentID(m.Id))
	}
}
