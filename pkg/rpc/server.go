package rpc

import (
	"context"
	"fmt"
	"reflect"

	"github.com/golang/protobuf/proto"
	"go.uber.org/zap"
)

// NewServer ...
func NewServer(logger *zap.Logger) *Server {
	return &Server{
		ServiceDispatcher: NewServiceDispatcher(logger),
	}
}

// Server ...
type Server struct {
	*ServiceDispatcher
	transport Transport
}

// Listen ...
func (s *Server) Listen(ctx context.Context, dialer Dialer) error {
	_, err := dialer.Dial(ctx, s.ServiceDispatcher)
	if err != nil {
		return err
	}

	<-ctx.Done()
	return ctx.Err()
}

// NewServiceDispatcher ...
func NewServiceDispatcher(logger *zap.Logger) *ServiceDispatcher {
	return &ServiceDispatcher{
		logger:  logger,
		methods: map[string]reflect.Value{},
	}
}

// ServiceDispatcher ...
type ServiceDispatcher struct {
	logger  *zap.Logger
	methods map[string]reflect.Value
}

// RegisterMethod ...
func (h *ServiceDispatcher) RegisterMethod(name string, method interface{}) {
	h.methods[name] = reflect.ValueOf(method)
}

// Dispatch ...
func (h *ServiceDispatcher) Dispatch(call *CallIn) {
	switch call.Method() {
	case cancelMethod:
		h.cancel(call)
	default:
		h.call(call)
	}
}

func (h *ServiceDispatcher) cancel(call *CallIn) {
	if parent, ok := call.Parent().(*CallIn); ok {
		parent.Cancel()
	}
}

func (h *ServiceDispatcher) call(call *CallIn) {
	defer func() {
		if err := recoverError(recover()); err != nil {
			h.logger.Error("call handler panicked", zap.Error(err), zap.Stack("stack"))
			call.returnError(err)
		}
	}()

	arg, err := call.Argument()
	if err != nil {
		call.returnError(err)
		return
	}

	method, ok := h.methods[call.Method()]
	if !ok {
		call.returnError(fmt.Errorf("method not found: %s", call.Method()))
		return
	}

	rs := method.Call([]reflect.Value{reflect.ValueOf(call.Context()), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		call.returnUndefined()
	} else if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		call.returnError(err)
	} else if r := rs[0]; r.Kind() == reflect.Chan {
		call.returnStream(r.Interface())
	} else if r, ok := rs[0].Interface().(proto.Message); ok {
		call.returnValue(r)
	}
}
