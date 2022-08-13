// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"reflect"

	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"golang.org/x/sync/errgroup"
	"google.golang.org/protobuf/proto"
)

var (
	opCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_dao_op_count",
		Help: "The total number of dao ops",
	}, []string{"type", "method"})
	opErrCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_dao_op_err_count",
		Help: "The total number of dao errors",
	}, []string{"type", "method"})
	opDurationMs = promauto.NewHistogramVec(prometheus.HistogramOpts{
		Name: "strims_dao_op_duration_ms",
		Help: "The run time of dao ops",
	}, []string{"type", "method"})
	secondaryIndexGetAllCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_dao_secondary_index_get_all_count",
		Help: "The total number of dao secondary index scans",
	}, []string{"type", "namespace"})
	secondaryIndexGetAllErrCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_dao_secondary_index_get_all_err_count",
		Help: "The total number of dao secondary index scan errors",
	}, []string{"type", "namespace"})
	secondaryIndexGetAllDurationMs = promauto.NewHistogramVec(prometheus.HistogramOpts{
		Name: "strims_dao_secondary_index_get_all_duration_ms",
		Help: "The run time of dao secondary index scans",
	}, []string{"type", "namespace"})
	writeThroughCacheReadCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_dao_write_through_cache_read_count",
		Help: "The total number of dao write through cache reads",
	}, []string{"type", "status"})
)

var Logger = zap.NewNop()

func wrapError(method, t string, err error) error {
	if err == nil {
		return nil
	}
	return fmt.Errorf("kv %s[%s]: %w", method, t, err)
}

func typeName[T any]() string {
	var t T
	return reflect.TypeOf(t).String()
}

func observeDurationMs(obs prometheus.Observer) func() {
	start := timeutil.Now()
	return func() {
		obs.Observe(float64(timeutil.Since(start).Milliseconds()))
	}
}

type EventEmitter interface {
	Emit(e proto.Message)
}

type EventEmitterFunc func(e proto.Message)

func (f EventEmitterFunc) Emit(e proto.Message) {
	f(e)
}

type changeObserverFunc[V any, T Record[V]] func(m, p T) proto.Message
type deleteObserverFunc[V any, T Record[V]] func(m T) proto.Message

func changeObserver[V any, T Record[V]](t changeObserverFunc[V, T]) setHook[V, T] {
	return func(s kv.RWStore, m, p T) error {
		if e, ok := s.(EventEmitter); ok {
			e.Emit(t(m, p))
		}
		return nil
	}
}

func deleteObserver[V any, T Record[V]](t deleteObserverFunc[V, T]) deleteHook[V, T] {
	return func(s kv.RWStore, m T) error {
		if e, ok := s.(EventEmitter); ok {
			e.Emit(t(m))
		}
		return nil
	}
}

type setHook[V any, T Record[V]] func(s kv.RWStore, m, p T) error
type deleteHook[V any, T Record[V]] func(s kv.RWStore, m T) error

type Record[V any] interface {
	proto.Message
	*V
}

type SingletonOptions[V any, T Record[V]] struct {
	DefaultValue  T
	ObserveChange changeObserverFunc[V, T]
}

func NewSingleton[V any, T Record[V]](ns namespace, opt *SingletonOptions[V, T]) *Singleton[V, T] {
	if opt == nil {
		opt = &SingletonOptions[V, T]{}
	}

	t := &Singleton[V, T]{
		ns:           ns,
		name:         typeName[V](),
		defaultValue: opt.DefaultValue,

		getCount:            opCount.WithLabelValues(typeName[V](), "get"),
		getErrCount:         opErrCount.WithLabelValues(typeName[V](), "get"),
		getDurationMs:       opDurationMs.WithLabelValues(typeName[V](), "get"),
		setCount:            opCount.WithLabelValues(typeName[V](), "set"),
		setErrCount:         opErrCount.WithLabelValues(typeName[V](), "set"),
		setDurationMs:       opDurationMs.WithLabelValues(typeName[V](), "set"),
		transformCount:      opCount.WithLabelValues(typeName[V](), "transform"),
		transformErrCount:   opErrCount.WithLabelValues(typeName[V](), "transform"),
		transformDurationMs: opDurationMs.WithLabelValues(typeName[V](), "transform"),
	}

	if opt.ObserveChange != nil {
		t.setHooks = append(t.setHooks, changeObserver(opt.ObserveChange))
	}

	return t
}

type Singleton[V any, T Record[V]] struct {
	ns           namespace
	name         string
	defaultValue T
	setHooks     []setHook[V, T]

	getCount            prometheus.Counter
	getErrCount         prometheus.Counter
	getDurationMs       prometheus.Observer
	setCount            prometheus.Counter
	setErrCount         prometheus.Counter
	setDurationMs       prometheus.Observer
	transformCount      prometheus.Counter
	transformErrCount   prometheus.Counter
	transformDurationMs prometheus.Observer
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
	t.getCount.Inc()
	defer observeDurationMs(t.getDurationMs)()

	v = v.ProtoReflect().New().Interface().(T)
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(t.ns.String(), v)
	})
	if err == kv.ErrRecordNotFound && t.defaultValue != nil {
		return proto.Clone(t.defaultValue).(T), nil
	}
	if err != nil {
		t.getErrCount.Inc()
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
	t.setCount.Inc()
	defer observeDurationMs(t.setDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if proto.Equal(p, v) {
			return nil
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.String(), v)
	})
	if err != nil {
		t.setErrCount.Inc()
		return wrapError("Singleton.Set", t.name, err)
	}
	return
}

func (t *Singleton[V, T]) runSetHooks(tx kv.RWTx, v T, p T) error {
	for _, h := range t.setHooks {
		if err := h(tx, v, p); err != nil {
			return err
		}
	}
	return nil
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
	t.transformCount.Inc()
	defer observeDurationMs(t.transformDurationMs)()

	err = s.Update(func(tx kv.RWTx) (err error) {
		p, err := t.Get(tx)
		if errors.Is(err, kv.ErrRecordNotFound) {
			p = new(V)
		} else if err != nil {
			return err
		}
		v = proto.Clone(p).(T)
		if err := fn(v); err != nil {
			return err
		}
		if proto.Equal(p, v) {
			return nil
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.String(), v)
	})
	if err != nil {
		t.transformErrCount.Inc()
		return nil, wrapError("Singleton.Transform", t.name, err)
	}
	return
}

type TableRecord[V any] interface {
	Record[V]
	GetId() uint64
}

type TableOptions[V any, T TableRecord[V]] struct {
	ObserveChange changeObserverFunc[V, T]
	ObserveDelete deleteObserverFunc[V, T]
}

func NewTable[V any, T TableRecord[V]](ns namespace, opt *TableOptions[V, T]) *Table[V, T] {
	if opt == nil {
		opt = &TableOptions[V, T]{}
	}

	t := &Table[V, T]{
		ns:   ns,
		name: typeName[V](),

		getCount:            opCount.WithLabelValues(typeName[V](), "get"),
		getErrCount:         opErrCount.WithLabelValues(typeName[V](), "get"),
		getDurationMs:       opDurationMs.WithLabelValues(typeName[V](), "get"),
		getAllCount:         opCount.WithLabelValues(typeName[V](), "getAll"),
		getAllErrCount:      opErrCount.WithLabelValues(typeName[V](), "getAll"),
		getAllDurationMs:    opDurationMs.WithLabelValues(typeName[V](), "getAll"),
		insertCount:         opCount.WithLabelValues(typeName[V](), "insert"),
		insertErrCount:      opErrCount.WithLabelValues(typeName[V](), "insert"),
		insertDurationMs:    opDurationMs.WithLabelValues(typeName[V](), "insert"),
		updateCount:         opCount.WithLabelValues(typeName[V](), "update"),
		updateErrCount:      opErrCount.WithLabelValues(typeName[V](), "update"),
		updateDurationMs:    opDurationMs.WithLabelValues(typeName[V](), "update"),
		upsertCount:         opCount.WithLabelValues(typeName[V](), "upsert"),
		upsertErrCount:      opErrCount.WithLabelValues(typeName[V](), "upsert"),
		upsertDurationMs:    opDurationMs.WithLabelValues(typeName[V](), "upsert"),
		transformCount:      opCount.WithLabelValues(typeName[V](), "transform"),
		transformErrCount:   opErrCount.WithLabelValues(typeName[V](), "transform"),
		transformDurationMs: opDurationMs.WithLabelValues(typeName[V](), "transform"),
		deleteCount:         opCount.WithLabelValues(typeName[V](), "delete"),
		deleteErrCount:      opErrCount.WithLabelValues(typeName[V](), "delete"),
		deleteDurationMs:    opDurationMs.WithLabelValues(typeName[V](), "delete"),
	}

	if opt.ObserveChange != nil {
		t.setHooks = append(t.setHooks, changeObserver(opt.ObserveChange))
	}
	if opt.ObserveDelete != nil {
		t.deleteHooks = append(t.deleteHooks, deleteObserver(opt.ObserveDelete))
	}

	return t
}

type Table[V any, T TableRecord[V]] struct {
	ns          namespace
	name        string
	setHooks    []setHook[V, T]
	deleteHooks []deleteHook[V, T]

	getCount            prometheus.Counter
	getErrCount         prometheus.Counter
	getDurationMs       prometheus.Observer
	getAllCount         prometheus.Counter
	getAllErrCount      prometheus.Counter
	getAllDurationMs    prometheus.Observer
	insertCount         prometheus.Counter
	insertErrCount      prometheus.Counter
	insertDurationMs    prometheus.Observer
	updateCount         prometheus.Counter
	updateErrCount      prometheus.Counter
	updateDurationMs    prometheus.Observer
	upsertCount         prometheus.Counter
	upsertErrCount      prometheus.Counter
	upsertDurationMs    prometheus.Observer
	transformCount      prometheus.Counter
	transformErrCount   prometheus.Counter
	transformDurationMs prometheus.Observer
	deleteCount         prometheus.Counter
	deleteErrCount      prometheus.Counter
	deleteDurationMs    prometheus.Observer
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
	t.getCount.Inc()
	defer observeDurationMs(t.getDurationMs)()

	v = new(V)
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(t.ns.Format(k), v)
	})
	if err != nil {
		t.getErrCount.Inc()
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
	t.getAllCount.Inc()
	defer observeDurationMs(t.getAllDurationMs)()

	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(t.ns.FormatPrefix(), &vs)
	})
	if err != nil {
		t.getAllErrCount.Inc()
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
	t.insertCount.Inc()
	defer observeDurationMs(t.insertDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		var p T
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		t.insertErrCount.Inc()
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
	t.updateCount.Inc()
	defer observeDurationMs(t.updateDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, v.GetId())
		if err != nil {
			return err
		}
		if proto.Equal(p, v) {
			return nil
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		t.updateErrCount.Inc()
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
	t.upsertCount.Inc()
	defer observeDurationMs(t.upsertDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, v.GetId())
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if proto.Equal(p, v) {
			return nil
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		t.upsertErrCount.Inc()
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
	t.transformCount.Inc()
	defer observeDurationMs(t.transformDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		p, err := t.Get(tx, id)
		v = proto.Clone(p).(T)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if err := fn(v); err != nil {
			return err
		}
		if proto.Equal(p, v) {
			return nil
		}
		if err := t.runSetHooks(tx, v, p); err != nil {
			return err
		}
		return tx.Put(t.ns.Format(v.GetId()), v)
	})
	if err != nil {
		t.transformErrCount.Inc()
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
	t.deleteCount.Inc()
	defer observeDurationMs(t.deleteDurationMs)()

	err = s.Update(func(tx kv.RWTx) error {
		if err := t.runDeleteHooks(tx, k); err != nil {
			return err
		}
		return tx.Delete(t.ns.Format(k))
	})
	if err != nil {
		t.deleteErrCount.Inc()
		return wrapError("Table.Delete", t.name, err)
	}
	return
}

func (t *Table[V, T]) runSetHooks(tx kv.RWTx, v T, p T) error {
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

func byteIdentity(b []byte) []byte { return b }

type ManyToOneOptions struct {
	CascadeDelete bool
}

func ManyToOne[AV, BV any, AT TableRecord[AV], BT TableRecord[BV]](ns namespace, a *Table[AV, AT], b *Table[BV, BT], key func(m AT) uint64, opt *ManyToOneOptions) (func(s kv.Store, id uint64) ([]AT, error), func(s kv.Store, v BT) ([]AT, error), func(s kv.Store, v AT) (BT, error)) {
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

	byKey := NewSecondaryIndex(ns, a, key, idKey)

	getAllAByBID := func(s kv.Store, id uint64) ([]AT, error) { return byKey.GetAll(s, id) }
	getAllAByB := func(s kv.Store, m BT) ([]AT, error) { return getAllAByBID(s, m.GetId()) }
	getBByA := func(s kv.Store, m AT) (BT, error) { return b.Get(s, key(m)) }

	return getAllAByBID, getAllAByB, getBByA
}

func idKey(id uint64) []byte {
	b := make([]byte, 8)
	binary.LittleEndian.PutUint64(b, id)
	return b
}

func NewSecondaryIndex[V any, T TableRecord[V], K any](ns namespace, t *Table[V, T], key func(m T) K, formatKey func(k K) []byte) *SecondaryIndex[V, T, K] {
	t.deleteHooks = append(t.deleteHooks, func(s kv.RWStore, m T) (err error) {
		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.DeleteHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Uint64("id", m.GetId()),
					zap.Stringer("ns", ns),
					zap.String("key", hashSecondaryIndexKey(formatKey(key(m)), s)),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		return s.Update(func(tx kv.RWTx) error {
			return DeleteSecondaryIndex(tx, ns, formatKey(key(m)), m.GetId())
		})
	})
	t.setHooks = append(t.setHooks, func(s kv.RWStore, m, p T) (err error) {
		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.SetHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Uint64("id", m.GetId()),
					zap.Stringer("ns", ns),
					zap.String("key", hashSecondaryIndexKey(formatKey(key(m)), s)),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		return s.Update(func(tx kv.RWTx) error {
			mk := formatKey(key(m))
			if p != nil {
				pk := formatKey(key(p))
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

	return &SecondaryIndex[V, T, K]{
		ns: ns,
		t:  t,
		k:  formatKey,

		getAllCount:         secondaryIndexGetAllCount.WithLabelValues(typeName[V](), ns.String()),
		getAllErrCount:      secondaryIndexGetAllErrCount.WithLabelValues(typeName[V](), ns.String()),
		getAllDurationMs:    secondaryIndexGetAllDurationMs.WithLabelValues(typeName[V](), ns.String()),
		getAllIDsCount:      secondaryIndexGetAllCount.WithLabelValues(typeName[V](), ns.String()),
		getAllIDsErrCount:   secondaryIndexGetAllErrCount.WithLabelValues(typeName[V](), ns.String()),
		getAllIDsDurationMs: secondaryIndexGetAllDurationMs.WithLabelValues(typeName[V](), ns.String()),
	}
}

type SecondaryIndex[V any, T TableRecord[V], K any] struct {
	ns namespace
	t  *Table[V, T]
	k  func(k K) []byte

	getAllCount         prometheus.Counter
	getAllErrCount      prometheus.Counter
	getAllDurationMs    prometheus.Observer
	getAllIDsCount      prometheus.Counter
	getAllIDsErrCount   prometheus.Counter
	getAllIDsDurationMs prometheus.Observer
}

func (idx *SecondaryIndex[V, T, K]) GetAll(s kv.Store, k K) (vs []T, err error) {
	kb := idx.k(k)
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.GetAll"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", idx.t.name),
				zap.Stringer("ns", idx.ns),
				zap.String("key", hashSecondaryIndexKey(kb, s)),
				zap.Int("records", len(vs)),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}
	idx.getAllCount.Inc()
	defer observeDurationMs(idx.getAllDurationMs)()

	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, idx.ns, kb)
		if err != nil {
			return err
		}

		for _, id := range ids {
			v, err := idx.t.Get(tx, id)
			if err != nil {
				return err
			}
			vs = append(vs, v)
		}
		return nil
	})
	if err != nil {
		idx.getAllErrCount.Inc()
	}
	return
}

func (idx *SecondaryIndex[V, T, K]) GetAllIDs(s kv.Store, k K) (ids []uint64, err error) {
	kb := idx.k(k)
	if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "SecondaryIndex.GetAllIDs"); ce != nil {
		defer func() {
			ce.Write(
				zap.String("type", idx.t.name),
				zap.Stringer("ns", idx.ns),
				zap.String("key", hashSecondaryIndexKey(kb, s)),
				zap.Int("records", len(ids)),
				zap.Duration("duration", ts.Elapsed()),
				zap.Error(err),
			)
		}()
	}
	idx.getAllIDsCount.Inc()
	defer observeDurationMs(idx.getAllIDsDurationMs)()

	err = s.View(func(tx kv.Tx) error {
		ids, err = ScanSecondaryIndex(tx, idx.ns, kb)
		return err
	})
	if err != nil {
		idx.getAllIDsErrCount.Inc()
	}
	return
}

var ErrUniqueConstraintViolated = errors.New("unique constraint violated")

type UniqueIndexOptions[V any, T TableRecord[V]] struct {
	OnConflict func(s kv.RWStore, t *Table[V, T], m, p T) error
}

func NewUniqueIndex[V any, T TableRecord[V], K any](ns namespace, t *Table[V, T], key func(m T) K, formatKey func(k K) []byte, opt *UniqueIndexOptions[V, T]) *UniqueIndex[V, T, K] {
	if opt == nil {
		opt = &UniqueIndexOptions[V, T]{}
	}

	t.setHooks = append(t.setHooks, func(s kv.RWStore, m, p T) (err error) {
		k := formatKey(key(m))

		if ce, ts := logutil.CheckWithTimer(Logger, zapcore.DebugLevel, "UniqueIndex.SetHook"); ce != nil {
			defer func() {
				ce.Write(
					zap.String("type", t.name),
					zap.Stringer("ns", ns),
					zap.String("key", hashSecondaryIndexKey(k, s)),
					zap.Duration("duration", ts.Elapsed()),
					zap.Error(err),
				)
			}()
		}

		if p != nil && bytes.Equal(k, formatKey(key(p))) {
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

	return &UniqueIndex[V, T, K]{NewSecondaryIndex(ns, t, key, formatKey)}
}

type UniqueIndex[V any, T TableRecord[V], K any] struct {
	i *SecondaryIndex[V, T, K]
}

func (idx *UniqueIndex[V, T, K]) Get(s kv.Store, k K) (v T, err error) {
	vs, err := idx.i.GetAll(s, k)
	if err != nil {
		return v, err
	}
	if len(vs) == 0 {
		return v, kv.ErrRecordNotFound
	}
	return vs[0], nil
}

func (idx *UniqueIndex[V, T, K]) GetID(s kv.Store, k K) (v uint64, err error) {
	ids, err := idx.i.GetAllIDs(s, k)
	if err != nil {
		return v, err
	}
	if len(ids) == 0 {
		return v, kv.ErrRecordNotFound
	}
	return ids[0], nil
}

func (idx *UniqueIndex[V, T, K]) GetMany(s kv.Store, ks ...K) (vs []T, err error) {
	vs = make([]T, len(ks))
	var eg errgroup.Group
	for i, k := range ks {
		i, k := i, k
		eg.Go(func() (err error) {
			vs[i], err = idx.Get(s, k)
			return
		})
	}
	err = eg.Wait()
	return
}

func (idx *UniqueIndex[V, T, K]) GetManyIDs(s kv.Store, ks ...K) (vs []uint64, err error) {
	vs = make([]uint64, len(ks))
	var eg errgroup.Group
	for i, k := range ks {
		i, k := i, k
		eg.Go(func() (err error) {
			vs[i], err = idx.GetID(s, k)
			return
		})
	}
	err = eg.Wait()
	return
}

func (idx *UniqueIndex[V, T, K]) Delete(s kv.RWStore, ks ...K) error {
	return s.Update(func(tx kv.RWTx) error {
		ids, err := idx.GetManyIDs(tx, ks...)
		if err != nil {
			return err
		}
		for _, id := range ids {
			if err := idx.i.t.Delete(tx, id); err != nil {
				return err
			}
		}
		return nil
	})
}
