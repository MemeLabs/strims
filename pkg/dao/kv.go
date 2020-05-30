package dao

import (
	"errors"
	"reflect"

	"google.golang.org/protobuf/proto"
)

// ErrRecordNotFound ...
var ErrRecordNotFound = errors.New("record not found")

// Store ...
type Store interface {
	CreateStoreIfNotExists(table string) error
	DeleteStore(table string) error
	View(table string, fn func(tx Tx) error) error
	Update(table string, fn func(tx Tx) error) error
}

// Tx ...
type Tx interface {
	Put(key string, value []byte) error
	Get(key string) ([]byte, error)
	Delete(key string) error
	ScanPrefix(prefix string) ([][]byte, error)
}

func put(tx Tx, sk *StorageKey, key string, m proto.Message) error {
	b, err := proto.Marshal(m)
	if err != nil {
		return err
	}
	b, err = sk.Seal(b)
	if err != nil {
		return err
	}
	return tx.Put(key, b)
}

func get(tx Tx, sk *StorageKey, key string, m proto.Message) error {
	b, err := tx.Get(key)
	if err != nil {
		return err
	}
	b, err = sk.Open(b)
	if err != nil {
		return err
	}
	return proto.Unmarshal(b, m)
}

var protoMessage proto.Message
var protoMessageType = reflect.TypeOf(protoMessage)

// read from the tx values from keys matching prefix and append them to the
// *[]*proto.Message
func scanPrefix(tx Tx, sk *StorageKey, prefix string, messages interface{}) error {
	bs, err := tx.ScanPrefix(prefix)
	if err != nil {
		return err
	}

	mv := reflect.ValueOf(messages).Elem()
	messages = mv.Interface()

	for _, b := range bs {
		b, err = sk.Open(b)
		if err != nil {
			return err
		}
		messages, err = appendUnmarshalled(messages, b)
		if err != nil {
			return err
		}
	}

	mv.Set(reflect.ValueOf(messages))

	return nil
}

// unmarshalMessages appends proto.Message elements unmarshalled from the
// supplied byte slices
func appendUnmarshalled(messages interface{}, bufs ...[]byte) (interface{}, error) {
	mt := reflect.TypeOf(messages).Elem().Elem()
	mv := reflect.ValueOf(messages)
	for _, b := range bufs {
		m := reflect.New(mt)
		if err := proto.Unmarshal(b, m.Interface().(proto.Message)); err != nil {
			return nil, err
		}
		mv = reflect.Append(mv, m)
	}

	return mv.Interface(), nil
}

func exists(tx Tx, key string) (bool, error) {
	_, err := tx.Get(key)
	if err == ErrRecordNotFound {
		return false, nil
	}
	return true, err
}
