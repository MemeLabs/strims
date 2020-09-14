package rpc

import (
	"context"
	"fmt"
	"io"
	"reflect"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"go.uber.org/zap"
)

// NewHost ...
func NewHost(logger *zap.Logger, service interface{}) *Host {
	return &Host{
		logger:  logger,
		service: service,
	}
}

// Host ...
type Host struct {
	logger  *zap.Logger
	service interface{}
}

// Listen starts reading incoming calls
func (h *Host) Listen(ctx context.Context, rw io.ReadWriter) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	c := &conn{w: rw}

	for {
		m, err := readCall(rw)
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

		if err := recoverError(recover()); err != nil {
			h.logger.Debug("call handler panicked", zap.Error(err), zap.Stack("stack"))

			e := &pb.Error{Message: err.Error()}
			if err := call(ctx, c, callbackMethod, e, withParentID(m.Id)); err != nil {
				h.logger.Debug("call failed", zap.Error(err))
			}
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
		if err := call(ctx, c, callbackMethod, e, withParentID(m.Id)); err != nil {
			h.logger.Debug("call failed", zap.Error(err))
		}
		return
	}

	rs := method.Call([]reflect.Value{reflect.ValueOf(ctx), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		if err := call(ctx, c, callbackMethod, &pb.Undefined{}, withParentID(m.Id)); err != nil {
			h.logger.Debug("call failed", zap.Error(err))
		}
		return
	}

	if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		if err := call(ctx, c, callbackMethod, &pb.Error{Message: err.Error()}, withParentID(m.Id)); err != nil {
			h.logger.Debug("call failed", zap.Error(err))
		}
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
				if err := call(ctx, c, callbackMethod, &pb.Close{}, withParentID(m.Id)); err != nil {
					h.logger.Debug("call failed", zap.Error(err))
				}
				return
			}
			if err := call(ctx, c, callbackMethod, v.Interface().(proto.Message), withParentID(m.Id)); err != nil {
				h.logger.Debug("call failed", zap.Error(err))
			}
		}
	}

	if a, ok := rs[0].Interface().(proto.Message); ok {
		if err := call(ctx, c, callbackMethod, a, withParentID(m.Id)); err != nil {
			h.logger.Debug("call failed", zap.Error(err))
		}
	}
}
