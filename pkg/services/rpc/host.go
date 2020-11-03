package rpc

import (
	"context"
	"errors"
	"fmt"
	"reflect"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/golang/protobuf/proto"
	"github.com/golang/protobuf/ptypes/any"
	"go.uber.org/zap"
)

// NewHost ...
func NewHost(logger *zap.Logger, client *vpn.Client, key *pb.Key, salt []byte) (*Host, error) {
	port, err := client.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	s := &Host{
		ctx:      ctx,
		cancel:   cancel,
		logger:   logger,
		client:   client,
		port:     port,
		services: make(map[string]interface{}),
	}

	addr := &HostAddr{
		HostID: client.Host.VNIC().ID(),
		Port:   port,
	}
	if err := PublishHostAddr(ctx, client, key, salt, addr); err != nil {
		return nil, err
	}

	if err := s.client.Network.SetHandler(port, s); err != nil {
		cancel()
		return nil, err
	}

	return s, nil
}

// Host ...
type Host struct {
	ctx      context.Context
	cancel   context.CancelFunc
	logger   *zap.Logger
	client   *vpn.Client
	port     uint16
	services map[string]interface{}
}

// RegisterService ...
func (s *Host) RegisterService(name string, service interface{}) {
	s.services[name] = service
}

// HandleMessage ...
func (s *Host) HandleMessage(msg *vpn.Message) (bool, error) {
	m := &pb.Call{}
	if err := proto.Unmarshal(msg.Body, m); err != nil {
		return false, nil
	}

	addr := &HostAddr{
		HostID: msg.Trailers[0].HostID,
		Port:   msg.Header.SrcPort,
	}
	go s.handleCall(addr, m)

	return false, nil
}

var nilValue = reflect.ValueOf(nil)

func (s *Host) findMethod(path string) (reflect.Value, error) {
	parts := strings.SplitN(path, "/", 2)
	if len(parts) != 2 {
		return nilValue, errors.New("invalid method format")
	}

	service, ok := s.services[parts[0]]
	if !ok {
		return nilValue, fmt.Errorf("service not found: %s", parts[0])
	}

	method := reflect.ValueOf(service).MethodByName(parts[1])
	if !method.IsValid() {
		return nilValue, fmt.Errorf("method not found: %s", path)
	}

	return method, nil
}

func (s *Host) handleCall(addr *HostAddr, m *pb.Call) {
	defer func() {
		if err := recoverError(recover()); err != nil {
			s.logger.Error("call handler panicked", zap.Error(err), zap.Stack("stack"))

			e := &pb.Error{Message: err.Error()}
			if err := s.call(addr, m.Id, callbackMethod, e); err != nil {
				s.logger.Error("call failed", zap.Error(err))
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

	method, err := s.findMethod(m.Method)
	if err != nil {
		e := &pb.Error{Message: err.Error()}
		if err := s.call(addr, m.Id, callbackMethod, e); err != nil {
			s.logger.Debug("call failed", zap.Error(err))
		}
		return
	}

	rs := method.Call([]reflect.Value{reflect.ValueOf(s.ctx), reflect.ValueOf(arg)})
	if len(rs) == 0 {
		if err := s.call(addr, m.Id, callbackMethod, &pb.Undefined{}); err != nil {
			s.logger.Debug("call failed", zap.Error(err))
		}
		return
	}

	if err, ok := rs[len(rs)-1].Interface().(error); ok && err != nil {
		if err := s.call(addr, m.Id, callbackMethod, &pb.Error{Message: err.Error()}); err != nil {
			s.logger.Debug("call failed", zap.Error(err))
		}
		return
	}

	if a, ok := rs[0].Interface().(proto.Message); ok {
		if err := s.call(addr, m.Id, callbackMethod, a); err != nil {
			s.logger.Debug("call failed", zap.Error(err))
		}
	}
}

func (s *Host) call(addr *HostAddr, parentID uint64, method string, req proto.Message) error {
	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	if err := b.Marshal(req); err != nil {
		return err
	}

	callID, err := dao.GenerateSnowflake()
	if err != nil {
		return err
	}

	call := &pb.Call{
		Id:       callID,
		ParentId: parentID,
		Method:   method,
		Argument: &any.Any{
			TypeUrl: anyURLPrefix + proto.MessageName(req),
			Value:   b.Bytes(),
		},
	}
	return s.client.Network.SendProto(addr.HostID, addr.Port, s.port, call)
}
