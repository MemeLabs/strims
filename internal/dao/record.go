package dao

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"google.golang.org/protobuf/proto"
)

var Logger = zap.NewNop()

func wrapError(method, t string, err error) error {
	if err == nil {
		return nil
	}
	return fmt.Errorf("kv %s[%s]: %w", method, t, err)
}

type SingletonRecord[V any] interface {
	proto.Message
	*V
}

type SingletonOptions[V any, T SingletonRecord[V]] struct {
	DefaultValue T
}

func NewSingleton[V any, T SingletonRecord[V]](ns namespace, opt *SingletonOptions[V, T]) *Singleton[V, T] {
	if opt == nil {
		opt = &SingletonOptions[V, T]{}
	}

	var temp V
	return &Singleton[V, T]{
		ns:   ns,
		name: reflect.TypeOf(temp).String(),
		opt:  opt,
	}
}

type Singleton[V any, T SingletonRecord[V]] struct {
	ns   namespace
	name string
	opt  *SingletonOptions[V, T]
}

func (t *Singleton[V, T]) Get(s kv.Store) (v T, err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Singleton.Get"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	v = v.ProtoReflect().New().Interface().(T)
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(t.ns.String(), v)
	})
	if err == kv.ErrRecordNotFound && t.opt.DefaultValue != nil {
		return proto.Clone(t.opt.DefaultValue).(T), nil
	}
	if err != nil {
		return nil, wrapError("Singleton.Get", t.name, err)
	}
	return
}

func (t *Singleton[V, T]) Set(s kv.RWStore, v T) (err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Singleton.Set"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		return tx.Put(t.ns.String(), v)
	})
	if err != nil {
		return wrapError("Singleton.Set", t.name, err)
	}
	return
}

func (t *Singleton[V, T]) Transform(s kv.RWStore, fn func(p T) error) (v T, err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Singleton.Transform"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) (err error) {
		v, err = t.Get(tx)
		if err == kv.ErrRecordNotFound {
			v = new(V)
		} else if err != nil {
			return err
		}
		if err := fn(v); err != nil {
			return err
		}
		return tx.Put(t.ns.String(), v)
	})
	if err != nil {
		return nil, wrapError("Singleton.Transform", t.name, err)
	}
	return
}

type Record[V any] interface {
	proto.Message
	GetId() uint64
	*V
}

func NewTable[V any, T Record[V]](ns namespace) *Table[V, T] {
	var temp V
	return &Table[V, T]{
		ns:   ns,
		name: reflect.TypeOf(temp).String(),
	}
}

type Table[V any, T Record[V]] struct {
	ns          namespace
	name        string
	setHooks    []func(s kv.RWStore, m T, p T) error
	deleteHooks []func(s kv.RWStore, m T) error
}

func (t *Table[V, T]) Get(s kv.Store, k uint64) (v T, err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Get"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", k),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	v = new(V)
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(t.ns.Format(k), v)
	})
	if err != nil {
		return nil, wrapError("Table.Get", t.name, err)
	}
	return
}

func (t *Table[V, T]) GetAll(s kv.Store) (vs []T, err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.GetAll"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Stringer("ns", t.ns),
				zap.Int("records", len(vs)),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(t.ns.FormatPrefix(), &vs)
	})
	if err != nil {
		return nil, wrapError("Table.GetAll", t.name, err)
	}
	return
}

func (t *Table[V, T]) Insert(s kv.RWStore, v T) (err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Insert"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", v.GetId()),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		var p T
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		return wrapError("Table.Insert", t.name, err)
	}
	return
}

func (t *Table[V, T]) Update(s kv.RWStore, v T) (err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Update"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", v.GetId()),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, v.GetId())
		if err != nil {
			return err
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		return wrapError("Table.Update", t.name, err)
	}
	return
}

func (t *Table[V, T]) Upsert(s kv.RWStore, v T) (err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Upsert"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", v.GetId()),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, v.GetId())
		if err != nil && err != kv.ErrRecordNotFound {
			return err
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		return wrapError("Table.Upsert", t.name, err)
	}
	return
}

func (t *Table[V, T]) Transform(s kv.RWStore, id uint64, fn func(p T) error) (v T, err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Transform"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", v.GetId()),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, id)
		v = proto.Clone(p).(T)
		if err != nil && err != kv.ErrRecordNotFound {
			return err
		}
		if err := fn(v); err != nil {
			return err
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		return nil, wrapError("Table.Transform", t.name, err)
	}
	return
}

func (t *Table[V, T]) Delete(s kv.RWStore, k uint64) (err error) {
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "Table.Delete"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", t.name),
				zap.Uint64("id", k),
				zap.Stringer("ns", t.ns),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}

	err = s.Update(func(tx kv.RWTx) error {
		if err := t.runDeleteHooks(tx, k); err != nil {
			return err
		}
		return tx.Delete(t.ns.Format(k))
	})
	if err != nil {
		return wrapError("Table.Delete", t.name, err)
	}
	return
}

func (t *Table[V, T]) runSetHooks(tx kv.RWTx, v T, p T) error {
	if len(t.setHooks) == 0 {
		return nil
	}

	for _, h := range t.setHooks {
		if err := h(tx, v, p); err != nil {
			return err
		}
	}
	return nil
}

func (t *Table[V, T]) runDeleteHooks(tx kv.RWTx, k uint64) error {
	if len(t.deleteHooks) == 0 {
		return nil
	}

	v, err := t.Get(tx, k)
	if err != nil {
		return err
	}
	for _, h := range t.deleteHooks {
		if err := h(tx, v); err != nil {
			return err
		}
	}
	return nil
}

type ManyToOneOptions struct {
	CascadeDelete bool
}

func ManyToOne[AV, BV any, AT Record[AV], BT Record[BV]](ns namespace, a *Table[AV, AT], b *Table[BV, BT], key func(m AT) uint64, opt *ManyToOneOptions) (func(s kv.Store, id uint64) ([]AT, error), func(s kv.Store, v BT) ([]AT, error), func(s kv.Store, v AT) (BT, error)) {
	if opt == nil {
		opt = &ManyToOneOptions{}
	}

	if opt.CascadeDelete {
		b.deleteHooks = append(b.deleteHooks, func(s kv.RWStore, m BT) error {
			return s.Update(func(tx kv.RWTx) (err error) {
				var ids []uint64

				if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "ManyToOne.CascadeDelete"); ce != nil {
					defer func() {
						ce.Write(
							zap.Object("from", zapcore.ObjectMarshalerFunc(func(e zapcore.ObjectEncoder) error {
								e.AddString("type", b.name)
								e.AddUint64("id", m.GetId())
								e.AddString("ns", b.ns.String())
								return nil
							})),
							zap.Object("to", zapcore.ObjectMarshalerFunc(func(e zapcore.ObjectEncoder) error {
								e.AddString("type", a.name)
								e.AddReflected("ids", ids)
								e.AddString("ns", a.ns.String())
								return nil
							})),
							zap.Duration("duration", ts.Elapsed()),
							zap.Error(err),
						)
					}()
				}

				ids, err = ScanSecondaryIndex(tx, ns, idKey(m.GetId()))
				if err != nil {
					return err
				}

				for _, id := range ids {
					if err := a.Delete(tx, id); err != nil {
						return err
					}
				}
				return nil
			})
		})
	}

	idk := func(m AT) []byte { return idKey(key(m)) }
	getAllAByKey := SecondaryIndex(ns, a, idk)

	getAllAByBID := func(s kv.Store, id uint64) ([]AT, error) { return getAllAByKey(s, idKey(id)) }
	getAllAByB := func(s kv.Store, m BT) ([]AT, error) { return getAllAByBID(s, m.GetId()) }
	getBByA := func(s kv.Store, m AT) (BT, error) { return b.Get(s, key(m)) }

	return getAllAByBID, getAllAByB, getBByA
}

func idKey(id uint64) []byte {
	b := make([]byte, 8)
	binary.LittleEndian.PutUint64(b, id)
	return b
}

func SecondaryIndex[V any, T Record[V]](ns namespace, t *Table[V, T], key func(m T) []byte) func(s kv.Store, k []byte) ([]T, error) {
	t.deleteHooks = append(t.deleteHooks, func(s kv.RWStore, m T) (err error) {
		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.DeleteHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Uint64("id", m.GetId()),
					zap.Stringer("ns", ns),
					zap.Stringer("key", secondaryKey{s, key(m)}),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		return s.Update(func(tx kv.RWTx) error {
			return DeleteSecondaryIndex(tx, ns, key(m), m.GetId())
		})
	})
	t.setHooks = append(t.setHooks, func(s kv.RWStore, m T, p T) (err error) {
		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.SetHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Uint64("id", m.GetId()),
					zap.Stringer("ns", ns),
					zap.Stringer("key", secondaryKey{s, key(m)}),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		return s.Update(func(tx kv.RWTx) error {
			mk := key(m)
			if p != nil {
				pk := key(p)
				if bytes.Equal(mk, pk) {
					return nil
				}
				if err := DeleteSecondaryIndex(tx, ns, pk, p.GetId()); err != nil {
					return err
				}
			}
			return SetSecondaryIndex(tx, ns, mk, m.GetId())
		})
	})

	return func(s kv.Store, k []byte) (vs []T, err error) {
		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.GetAll"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Stringer("ns", ns),
					zap.Stringer("key", secondaryKey{s, k}),
					zap.Int("records", len(vs)),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		err = s.View(func(tx kv.Tx) error {
			ids, err := ScanSecondaryIndex(tx, ns, k)
			if err != nil {
				return err
			}

			for _, id := range ids {
				v, err := t.Get(tx, id)
				if err != nil {
					return err
				}
				vs = append(vs, v)
			}
			return nil
		})
		return
	}
}

var ErrUniqueConstraintViolated = errors.New("unique constraint violated")

type UniqueIndexOptions[V any, T Record[V]] struct {
	OnConflict func(s kv.RWStore, t *Table[V, T], m, p T) error
}

func UniqueIndex[V any, T Record[V]](ns namespace, t *Table[V, T], key func(m T) []byte, opt *UniqueIndexOptions[V, T]) func(s kv.Store, k []byte) (T, error) {
	if opt == nil {
		opt = &UniqueIndexOptions[V, T]{}
	}

	t.setHooks = append(t.setHooks, func(s kv.RWStore, m T, p T) (err error) {
		k := key(m)

		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "UniqueIndex.SetHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Stringer("ns", ns),
					zap.Stringer("key", secondaryKey{s, k}),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		if p != nil && bytes.Equal(k, key(p)) {
			return nil
		}

		ids, err := ScanSecondaryIndex(s, ns, k)
		if err != nil {
			return err
		}
		if len(ids) == 0 {
			return nil
		}

		if opt.OnConflict == nil {
			return ErrUniqueConstraintViolated
		}
		c, err := t.Get(s, ids[0])
		if err != nil {
			return err
		}
		return opt.OnConflict(s, t, m, c)
	})

	get := SecondaryIndex(ns, t, key)
	return func(s kv.Store, k []byte) (v T, err error) {
		vs, err := get(s, k)
		if err != nil {
			return v, err
		}
		if len(vs) == 0 {
			return v, kv.ErrRecordNotFound
		}
		return vs[0], nil
	}
}
