// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"crypto/ed25519"
	"encoding/binary"
	"errors"
	"hash/crc32"
	"math"
	"reflect"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/pool"
	"google.golang.org/protobuf/proto"
)

// CRC32Message ...
func CRC32Message(m proto.Message) uint32 {
	e := &signableMessageEncoder{}
	rv := reflect.ValueOf(m)
	b := make([]byte, e.len(rv))
	e.encode(rv, b)
	return crc32.ChecksumIEEE(b)
}

// SignableMessage ...
type SignableMessage interface {
	proto.Message
	GetKey() []byte
	GetSignature() []byte
}

// SignMessage ...
func SignMessage(m SignableMessage, k *key.Key) error {
	mv := reflect.ValueOf(m).Elem()
	mv.FieldByName("Key").SetBytes(k.Public)

	b := marshalSignableMessage(m)
	defer pool.Put(b)

	switch k.Type {
	case key.KeyType_KEY_TYPE_ED25519:
		if len(k.Private) != ed25519.PrivateKeySize {
			return ErrInvalidKeyLength
		}
		mv.FieldByName("Signature").SetBytes(ed25519.Sign(k.Private, *b))
	default:
		return ErrUnsupportedKeyType
	}
	return nil
}

// VerifyMessage ...
func VerifyMessage(m SignableMessage) error {
	b := marshalSignableMessage(m)
	defer pool.Put(b)

	if len(m.GetKey()) != ed25519.PublicKeySize {
		return ErrInvalidKeyLength
	}
	if !ed25519.Verify(m.GetKey(), *b, m.GetSignature()) {
		return errors.New("invalid signature")
	}
	return nil
}

func marshalSignableMessage(m SignableMessage) *[]byte {
	if sig := m.GetSignature(); sig != nil {
		sigField := reflect.ValueOf(m).Elem().FieldByName("Signature")
		sigField.SetBytes(nil)
		defer sigField.SetBytes(sig)
	}

	e := &signableMessageEncoder{}
	rv := reflect.ValueOf(m)
	b := pool.Get(e.len(rv))
	e.encode(rv, *b)

	return b
}

// provides a simple stable binary format for protobuf types with ordered fields
type signableMessageEncoder struct{}

func (e *signableMessageEncoder) len(rv reflect.Value) int {
	switch rv.Kind() {
	case reflect.Bool, reflect.Int32, reflect.Int64, reflect.Uint32, reflect.Uint64, reflect.Float32, reflect.Float64:
		return int(rv.Type().Size())
	case reflect.Interface, reflect.Ptr:
		return e.len(rv.Elem())
	case reflect.Slice:
		et := rv.Type().Elem()
		switch et.Kind() {
		case reflect.Bool, reflect.Int32, reflect.Int64, reflect.Uint8, reflect.Uint32, reflect.Uint64, reflect.Float32, reflect.Float64:
			return int(et.Size()) * rv.Len()
		default:
			var n int
			for i := 0; i < rv.Len(); i++ {
				n += e.len(rv.Index(i))
			}
			return n
		}
	case reflect.String:
		return len([]byte(rv.String()))
	case reflect.Struct:
		var n int
		for i := 0; i < rv.NumField(); i++ {
			if fv := rv.Field(i); fv.CanInterface() {
				n += e.len(fv)
			}
		}
		return n
	case reflect.Invalid:
		return 0
	default:
		panic("cannot serialize value: " + rv.Kind().String())
	}
}

func (e *signableMessageEncoder) encode(rv reflect.Value, b []byte) int {
	var n int
	switch rv.Kind() {
	case reflect.Bool:
		if rv.Bool() {
			b[n] = 1
		} else {
			b[n] = 0
		}
		n++
	case reflect.Int32:
		binary.BigEndian.PutUint32(b[n:], uint32(rv.Int()))
		n += 4
	case reflect.Int64:
		binary.BigEndian.PutUint64(b[n:], uint64(rv.Int()))
		n += 8
	case reflect.Uint32:
		binary.BigEndian.PutUint32(b[n:], uint32(rv.Uint()))
		n += 4
	case reflect.Uint64:
		binary.BigEndian.PutUint64(b[n:], rv.Uint())
		n += 8
	case reflect.Float32:
		binary.BigEndian.PutUint32(b[n:], math.Float32bits(float32(rv.Float())))
		n += 4
	case reflect.Float64:
		binary.BigEndian.PutUint64(b[n:], math.Float64bits(rv.Float()))
		n += 8
	case reflect.Interface, reflect.Ptr:
		n += e.encode(rv.Elem(), b[n:])
	case reflect.Slice:
		et := rv.Type().Elem()
		switch et.Kind() {
		case reflect.Uint8:
			n += copy(b[n:], rv.Bytes())
		default:
			for i := 0; i < rv.Len(); i++ {
				n += e.encode(rv.Index(i), b[n:])
			}
		}
	case reflect.String:
		n += copy(b[n:], []byte(rv.String()))
	case reflect.Struct:
		for i := 0; i < rv.NumField(); i++ {
			if fv := rv.Field(i); fv.CanInterface() {
				n += e.encode(fv, b[n:])
			}
		}
	case reflect.Invalid:
	default:
		panic("cannot serialize value: " + rv.Kind().String())
	}
	return n
}
