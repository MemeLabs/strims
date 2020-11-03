package rpc

import (
	"errors"
	"fmt"
	"reflect"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"github.com/golang/protobuf/ptypes"
	"github.com/golang/protobuf/ptypes/any"
)

const (
	clientTimeout = time.Second * 10

	callbackMethod = "_CALLBACK"
	cancelMethod   = "_CANCEL"

	anyURLPrefix = "strims.gg/"
)

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
