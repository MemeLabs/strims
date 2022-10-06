package dao

import (
	"errors"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"golang.org/x/exp/maps"
	"google.golang.org/protobuf/proto"
)

type ReplicationEventLogTable struct {
	*Table[replicationv1.EventLog, *replicationv1.EventLog]
}

func (t ReplicationEventLogTable) Insert(s kv.RWStore, l *replicationv1.EventLog) error {
	l = proto.Clone(l).(*replicationv1.EventLog)
	for _, e := range l.Events {
		e.Record = nil
	}
	return t.Table.Insert(s, l)
}

func (t ReplicationEventLogTable) GetAllAfter(s kv.Store, checkpoint *daov1.VersionVector) ([]*replicationv1.EventLog, error) {
	var es []*replicationv1.EventLog
	err := s.View(func(tx kv.Tx) error {
		logs, err := ReplicationEventLogs.GetAll(s)
		if err != nil {
			return err
		}

		for _, l := range logs {
			if ts, ok := checkpoint.Value[l.ReplicaId]; ok && l.Timestamp >= ts {
				es = append(es, l)
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return es, nil
}

func (t ReplicationEventLogTable) GetCompressedDelta(s Store, checkpoint *daov1.VersionVector) ([]*replicationv1.EventLog, error) {
	var events []*replicationv1.EventLog

	err := s.View(func(tx kv.Tx) error {
		logs, err := t.GetAllAfter(tx, checkpoint)
		if err != nil {
			return err
		}
		replicaLogs := map[uint64][]*replicationv1.EventLog{}
		for _, l := range logs {
			replicaLogs[l.ReplicaId] = append(replicaLogs[l.ReplicaId], l)
		}

		var id, ts uint64
		for replicaID, ls := range replicaLogs {
			f := s.EventFilter(nil)
			for _, l := range ls {
				if l.Timestamp > ts {
					id = l.Id
					ts = l.Timestamp
				}

				for _, e := range l.Events {
					if err := f.AddEvent(tx, e); err != nil {
						if !errors.Is(err, errReplicatorNotFound) {
							return err
						}
						Logger.Warn(
							"omitting replication event",
							zap.Uint64("logID", l.Id),
							zap.Uint64("replicaID", l.ReplicaId),
							zap.Stringer("ns", namespace(e.Namespace)),
							zap.Uint64("id", e.Id),
							zap.Error(err),
						)
					}
				}
			}

			events = append(events, &replicationv1.EventLog{
				Id:        id,
				ReplicaId: replicaID,
				Timestamp: ts,
				Events:    f.Events(),
			})
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return events, nil
}

var ReplicationEventLogs = ReplicationEventLogTable{
	NewTable[replicationv1.EventLog](replicationLogNS, nil),
}

type ReplicationCheckpointTable struct {
	*Table[replicationv1.Checkpoint, *replicationv1.Checkpoint]
}

func (t ReplicationCheckpointTable) Merge(s kv.RWStore, v *replicationv1.Checkpoint) (*replicationv1.Checkpoint, error) {
	return t.Transform(s, v.Id, func(p *replicationv1.Checkpoint) error {
		proto.Merge(p, v)
		return nil
	})
}

func (t ReplicationCheckpointTable) MergeAll(s kv.RWStore, vs []*replicationv1.Checkpoint) ([]*replicationv1.Checkpoint, error) {
	var cs []*replicationv1.Checkpoint
	err := s.Update(func(tx kv.RWTx) error {
		for _, v := range vs {
			c, err := t.Merge(tx, v)
			if err != nil {
				return err
			}
			cs = append(cs, c)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return cs, nil
}

var ReplicationCheckpoints = ReplicationCheckpointTable{
	NewTable(
		replicationVersionNS,
		&TableOptions[replicationv1.Checkpoint, *replicationv1.Checkpoint]{
			OnChange: func(s kv.RWStore, m, p *replicationv1.Checkpoint) error {
				if p != nil {
					versionvector.Update(m.Version, p.Version)
				}
				return nil
			},
			ObserveChange: func(m, p *replicationv1.Checkpoint) proto.Message {
				return &replicationv1.CheckpointChangeEvent{Checkpoint: m}
			},
		},
	),
}

func NewReplicationCheckpointFromLogs(replicaID uint64, ls []*replicationv1.EventLog) *replicationv1.Checkpoint {
	c := &replicationv1.Checkpoint{
		Id:      replicaID,
		Version: versionvector.New(),
	}
	for _, l := range ls {
		c.Version.Value[l.ReplicaId] = l.Timestamp
	}
	return c
}

var _ ReplicatedRWTx = (*replicatedStoreTx)(nil)

type ReplicationEventFilter interface {
	Test(e *replicationv1.Event) bool
	AddEvent(s kv.Store, e *replicationv1.Event) error
	Events() []*replicationv1.Event
}

type Replicator interface {
	EventFilter() ReplicationEventFilter
	ApplyEvent(s kv.RWStore, e *replicationv1.Event) error
	Dump(s kv.Store) ([]*replicationv1.Event, error)
}

var replicators syncutil.Map[namespace, func() Replicator]

func NewReplicatedStore(s *ProfileStore) (*ReplicatedStore, error) {
	p, err := Profile.Get(s)
	if err != nil {
		return nil, err
	}

	rs := &ReplicatedStore{
		ProfileStore: s,
		replicators:  map[namespace]Replicator{},
		replicaID:    p.DeviceId,
	}

	replicators.Each(func(ns namespace, ctor func() Replicator) {
		rs.replicators[ns] = ctor()
	})

	return rs, nil
}

type ReplicatedStore struct {
	*ProfileStore
	replicators map[namespace]Replicator
	replicaID   uint64
}

func (s *ReplicatedStore) Update(fn func(tx kv.RWTx) error) (err error) {
	return s.ProfileStore.Update(func(tx kv.RWTx) error {
		ptx := &replicatedStoreTx{
			profileStoreTx: tx.(*profileStoreTx),
			replicaID:      s.ReplicaID(),
		}
		if err := fn(ptx); err != nil {
			return err
		}
		if len(ptx.events) != 0 {
			return s.commitEventLog(ptx, ptx.events)
		}
		return nil
	})
}

func (s *ReplicatedStore) commitEventLog(tx kv.RWTx, events []*replicationv1.Event) error {
	id, _, err := ProfileID.Incr(tx, 1)
	if err != nil {
		return err
	}

	ts := uint64(timeutil.Now().Unix())

	_, err = ReplicationCheckpoints.Merge(tx, &replicationv1.Checkpoint{
		Id: s.replicaID,
		Version: &daov1.VersionVector{
			Value: map[uint64]uint64{s.replicaID: ts},
		},
	})
	if err != nil {
		return err
	}

	l := &replicationv1.EventLog{
		Id:        id,
		ReplicaId: s.replicaID,
		Timestamp: ts,
		Events:    events,
	}
	tx.(EventEmitter).Emit(l)
	return ReplicationEventLogs.Insert(tx, l)
}

func (s *ReplicatedStore) ReplicaID() uint64 {
	return s.replicaID
}

func (s *ReplicatedStore) EventFilter(offset ReplicationEventFilter) ReplicationEventFilter {
	return newReplicationEventFilter(s.replicators, offset)
}

func (s *ReplicatedStore) Dump() ([]*replicationv1.Event, error) {
	var es []*replicationv1.Event
	err := s.View(func(tx kv.Tx) error {
		for _, r := range s.replicators {
			res, err := r.Dump(tx)
			if err != nil {
				return err
			}
			es = append(es, res...)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return es, nil
}

func (s *ReplicatedStore) applyEvents(tx kv.RWTx, es []*replicationv1.Event) error {
	for _, e := range es {
		m, ok := s.replicators[namespace(e.Namespace)]
		if !ok {
			return errors.New("wur replication meme?")
		}

		if err := m.ApplyEvent(tx, e); err != nil {
			return err
		}
	}
	return nil
}

func (s *ReplicatedStore) ApplyEvents(es []*replicationv1.Event, c *replicationv1.Checkpoint) (*replicationv1.Checkpoint, error) {
	err := s.ProfileStore.Update(func(tx kv.RWTx) (err error) {
		if err := s.applyEvents(tx, es); err != nil {
			return err
		}
		c, err = ReplicationCheckpoints.Merge(tx, c)
		return err
	})
	if err != nil {
		return nil, err
	}
	return c, nil
}

func (s *ReplicatedStore) ApplyEventLogs(ls []*replicationv1.EventLog) (*replicationv1.Checkpoint, error) {
	var c *replicationv1.Checkpoint
	err := s.ProfileStore.Update(func(tx kv.RWTx) error {
		c = NewReplicationCheckpointFromLogs(s.replicaID, ls)
		pls, err := ReplicationEventLogs.GetAllAfter(tx, c.Version)
		if err != nil {
			return err
		}

		of := s.EventFilter(nil)
		for _, l := range pls {
			for _, e := range l.Events {
				if err := of.AddEvent(tx, e); err != nil {
					return err
				}
			}
		}

		f := s.EventFilter(of)
		for _, l := range ls {
			for _, e := range l.Events {
				if err := f.AddEvent(tx, e); err != nil {
					return err
				}
			}
		}

		if err := s.applyEvents(tx, f.Events()); err != nil {
			return err
		}
		c, err = ReplicationCheckpoints.Merge(tx, c)
		if err != nil {
			return err
		}

		for _, l := range ls {
			if err := ReplicationEventLogs.Insert(tx, l); err != nil {
				return err
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return c, nil
}

var errReplicatorNotFound = errors.New("replicator not found")

func newReplicationEventFilter(replicators map[namespace]Replicator, offset ReplicationEventFilter) *replciationEventFilter {
	return &replciationEventFilter{
		replicators: replicators,
		filters:     map[namespace]ReplicationEventFilter{},
		offset:      offset,
	}
}

type replciationEventFilter struct {
	replicators map[namespace]Replicator
	filters     map[namespace]ReplicationEventFilter
	offset      ReplicationEventFilter
}

func (f *replciationEventFilter) filter(ns namespace) (ReplicationEventFilter, error) {
	if nf, ok := f.filters[ns]; ok {
		return nf, nil
	}
	if r, ok := f.replicators[ns]; ok {
		nf := r.EventFilter()
		f.filters[ns] = nf
		return nf, nil
	}
	// TODO: this should be a non-terminal error otherwise replication will break
	// when types are deprecated
	return nil, errReplicatorNotFound
}

func (f *replciationEventFilter) Test(e *replicationv1.Event) bool {
	if f.offset != nil && !f.offset.Test(e) {
		return false
	}
	if nf, ok := f.filters[namespace(e.Namespace)]; ok {
		return nf.Test(e)
	}
	return true
}

func (f *replciationEventFilter) AddEvent(s kv.Store, e *replicationv1.Event) error {
	if !f.Test(e) {
		return nil
	}
	nf, err := f.filter(namespace(e.Namespace))
	if err != nil {
		return err
	}
	return nf.AddEvent(s, e)
}

func (f *replciationEventFilter) Events() []*replicationv1.Event {
	var es []*replicationv1.Event
	for _, f := range f.filters {
		es = append(es, f.Events()...)
	}
	return es
}

type replicatedStoreTx struct {
	*profileStoreTx
	replicaID uint64
	events    []*replicationv1.Event
}

func (t *replicatedStoreTx) Update(fn func(tx kv.RWTx) error) error {
	return fn(t)
}

func (t *replicatedStoreTx) ReplicaID() uint64 {
	return t.replicaID
}

func (t *replicatedStoreTx) Replicate(e *replicationv1.Event) {
	t.events = append(t.events, e)
}

type ReplicatedRWTx interface {
	kv.RWTx
	ReplicaID() uint64
	Replicate(m *replicationv1.Event)
}

func LocalSetHook[V any, T Record[V]](fn func(s ReplicatedRWTx, m, p T) error) setHook[V, T] {
	return func(s kv.RWStore, m, p T) (err error) {
		if rs, ok := s.(ReplicatedRWTx); ok {
			return fn(rs, m, p)
		}
		return nil
	}
}

func LocalDeleteHook[V any, T Record[V]](fn func(s ReplicatedRWTx, p T) error) deleteHook[V, T] {
	return func(s kv.RWStore, p T) (err error) {
		if rs, ok := s.(ReplicatedRWTx); ok {
			return fn(rs, p)
		}
		return nil
	}
}

type ReplicatedRecord interface {
	GetVersion() *daov1.VersionVector
}

type ReplicatedTableRecord[V any] interface {
	TableRecord[V]
	ReplicatedRecord
}

type ReplicatedTableOptions[T ReplicatedRecord] struct {
	OnConflict func(s kv.RWStore, m, p T) error
	Extract    func(s kv.Store, m, p T) T
	Merge      func(s kv.RWStore, m, p T) T
}

func RegisterReplicatedTable[V any, T ReplicatedTableRecord[V]](t *Table[V, T], opt *ReplicatedTableOptions[T]) {
	opt = options.NewWithDefaults(opt, ReplicatedTableOptions[T]{
		OnConflict: func(s kv.RWStore, m, p T) error {
			if m.GetVersion().UpdatedAt < p.GetVersion().UpdatedAt {
				m, p = p, m
			}
			versionvector.Update(m.GetVersion(), p.GetVersion())
			return t.Update(s, m)
		},
		Extract: func(s kv.Store, m, p T) T { return m },
		Merge: func(s kv.RWStore, m, p T) T {
			versionvector.Update(m.GetVersion(), p.GetVersion())
			return m
		},
	})

	t.setHooks = append(t.setHooks, LocalSetHook(func(s ReplicatedRWTx, m, p T) (err error) {
		versionvector.Increment(m.GetVersion(), s.ReplicaID())

		m = proto.Clone(m).(T)
		b, err := proto.Marshal(opt.Extract(s, m, p))
		if err != nil {
			return err
		}
		s.Replicate(&replicationv1.Event{
			Namespace: int64(t.ns),
			Id:        m.GetId(),
			Version:   m.GetVersion(),
			Record:    b,
		})

		return nil
	}))

	t.deleteHooks = append(t.deleteHooks, LocalDeleteHook(func(s ReplicatedRWTx, p T) (err error) {
		versionvector.Increment(p.GetVersion(), s.ReplicaID())

		s.Replicate(&replicationv1.Event{
			Namespace: int64(t.ns),
			Id:        p.GetId(),
			Version:   p.GetVersion(),
			Delete:    true,
		})

		return nil
	}))

	replicators.Set(t.ns, func() Replicator {
		return &TableReplicator[V, T]{
			t:   t,
			opt: opt,
		}
	})
}

type TableReplicator[V any, T ReplicatedTableRecord[V]] struct {
	t   *Table[V, T]
	opt *ReplicatedTableOptions[T]
}

func (t *TableReplicator[V, T]) ApplyEvent(s kv.RWStore, e *replicationv1.Event) error {
	if e.Delete {
		return t.t.Delete(s, e.Id)
	}

	next := (T)(new(V))
	if err := proto.Unmarshal(e.Record, next); err != nil {
		return err
	}

	index, err := t.t.Get(s, next.GetId())
	if errors.Is(err, kv.ErrRecordNotFound) {
		return t.t.Insert(s, next)
	} else if err != nil {
		return err
	}

	d, ordered := versionvector.Compare(index.GetVersion(), next.GetVersion())
	if !ordered {
		return t.opt.OnConflict(s, next, index)
	} else if d < 0 {
		return t.t.Update(s, t.opt.Merge(s, next, index))
	}
	return nil
}

func (t *TableReplicator[V, T]) EventFilter() ReplicationEventFilter {
	return &TableReplicatorEventFilter[V, T]{
		t:      t.t,
		opt:    t.opt,
		events: map[uint64]*replicationv1.Event{},
	}
}

func (t *TableReplicator[V, T]) Dump(s kv.Store) ([]*replicationv1.Event, error) {
	rs, err := t.t.GetAll(s)
	if err != nil {
		return nil, err
	}

	es := make([]*replicationv1.Event, len(rs))
	for i, r := range rs {
		b, err := proto.Marshal(t.opt.Extract(s, r, nil))
		if err != nil {
			return nil, err
		}
		es[i] = &replicationv1.Event{
			Namespace: int64(t.t.ns),
			Id:        r.GetId(),
			Version:   r.GetVersion(),
			Record:    b,
		}
	}
	return es, nil
}

type TableReplicatorEventFilter[V any, T ReplicatedTableRecord[V]] struct {
	t      *Table[V, T]
	opt    *ReplicatedTableOptions[T]
	events map[uint64]*replicationv1.Event
}

func (t *TableReplicatorEventFilter[V, T]) Test(e *replicationv1.Event) bool {
	if index, ok := t.events[e.Id]; ok {
		if index.Delete {
			return false
		}

		d, ordered := versionvector.Compare(index.Version, e.Version)
		return !ordered || d < 0
	}
	return true
}

func (t *TableReplicatorEventFilter[V, T]) AddEvent(s kv.Store, e *replicationv1.Event) error {
	if index, ok := t.events[e.Id]; ok {
		if index.Delete {
			versionvector.Update(index.Version, e.Version)
			return nil
		}

		d, ordered := versionvector.Compare(index.Version, e.Version)
		if !ordered {
			return t.load(s, e.Id)
		} else if d > 0 {
			return nil
		}
	}

	if !e.Delete && e.Record == nil {
		return t.load(s, e.Id)
	}
	t.events[e.Id] = e
	return nil
}

func (t *TableReplicatorEventFilter[V, T]) load(s kv.Store, id uint64) error {
	e := &replicationv1.Event{
		Namespace: int64(t.t.ns),
		Id:        id,
	}

	r, err := t.t.Get(s, id)
	if errors.Is(err, kv.ErrRecordNotFound) {
		e.Version = versionvector.New()
		e.Delete = true
	} else if err == nil {
		e.Version = r.GetVersion()
		e.Record, err = proto.Marshal(t.opt.Extract(s, r, nil))
		if err != nil {
			return err
		}
	} else {
		return err
	}

	t.events[id] = e
	return nil
}

func (t *TableReplicatorEventFilter[V, T]) Events() []*replicationv1.Event {
	return maps.Values(t.events)
}
