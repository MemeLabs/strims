package dao

import (
	"errors"
	"sort"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/options"
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
		logs, err := ReplicationEventLogs.GetAll(tx)
		if err != nil {
			return err
		}

		for _, l := range logs {
			if v, ok := checkpoint.Value[l.Checkpoint.Id]; !ok || replicationEventLogLocalVersion(l) >= v {
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

func (t ReplicationEventLogTable) GarbageCollect(s kv.RWStore, checkpoint *daov1.VersionVector) (n int, err error) {
	err = s.Update(func(tx kv.RWTx) error {
		logs, err := ReplicationEventLogs.GetAll(tx)
		if err != nil {
			return err
		}

		for _, l := range logs {
			if v, ok := checkpoint.Value[l.Checkpoint.Id]; ok && replicationEventLogLocalVersion(l) < v {
				n++
				if err := t.Table.Delete(tx, l.Id); err != nil {
					return err
				}
			}
		}
		return nil
	})
	if err != nil {
		return 0, err
	}
	return
}

func (t ReplicationEventLogTable) GetCompressedDelta(s kv.RWStore, checkpoint *daov1.VersionVector) ([]*replicationv1.EventLog, error) {
	var cls []*replicationv1.EventLog
	err := s.View(func(tx kv.Tx) error {
		logs, err := t.GetAllAfter(tx, checkpoint)
		if err != nil {
			return err
		}
		replicaLogs := map[uint64][]*replicationv1.EventLog{}
		for _, l := range logs {
			replicaLogs[l.Checkpoint.Id] = append(replicaLogs[l.Checkpoint.Id], l)
		}

		for _, ls := range replicaLogs {
			sort.Slice(ls, func(i, j int) bool {
				return replicationEventLogLocalVersion(ls[i]) > replicationEventLogLocalVersion(ls[j])
			})
			var f ReplicationEventFilter
			for _, l := range ls {
				f = newReplicationEventFilter(f)
				for _, e := range l.Events {
					if err := f.AddEvent(tx, e); err != nil {
						if !errors.Is(err, ErrReplicatorNotFound) {
							return err
						}
						Logger.Warn(
							"omitting replication event",
							zap.Uint64("logID", l.Id),
							zap.Uint64("replicaID", l.Checkpoint.Id),
							zap.Stringer("ns", namespace(e.Namespace)),
							zap.Uint64("id", e.Id),
							zap.Error(err),
						)
					}
				}
				l.Events = f.Events()
			}
			for _, l := range ls {
				if len(l.Events) > 0 {
					cls = append(cls, l)
				}
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return cls, nil
}

var ReplicationEventLogs = ReplicationEventLogTable{
	NewTable[replicationv1.EventLog](replicationLogNS, nil),
}

type ReplicationCheckpointTable struct {
	*Table[replicationv1.Checkpoint, *replicationv1.Checkpoint]
}

func (t ReplicationCheckpointTable) Increment(s kv.RWStore, id uint64) (*replicationv1.Checkpoint, error) {
	return t.Transform(s, id, func(p *replicationv1.Checkpoint) error {
		if p.Version == nil {
			p.Id = id
			p.Version = versionvector.New()
		}
		versionvector.Increment(p.Version, id)
		return nil
	})
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
					versionvector.Upgrade(m.Version, p.Version)
				}
				return nil
			},
			ObserveChange: func(m, p *replicationv1.Checkpoint) proto.Message {
				return &replicationv1.CheckpointChangeEvent{Checkpoint: m}
			},
			ObserveDelete: func(m *replicationv1.Checkpoint) proto.Message {
				return &replicationv1.CheckpointDeleteEvent{Checkpoint: m}
			},
		},
	),
}

func newVersionVectorFromReplicationEventLogs(ls []*replicationv1.EventLog, update func(d *daov1.VersionVector, vs ...*daov1.VersionVector)) *daov1.VersionVector {
	switch len(ls) {
	case 0:
		return versionvector.New()
	case 1:
		return versionvector.New(ls[0].Checkpoint.Version)
	default:
		v := versionvector.New(ls[0].Checkpoint.Version)
		for i := 1; i < len(ls); i++ {
			update(v, ls[i].Checkpoint.Version)
		}
		return v
	}
}

func NewVersionVectorFromReplicationEventLogs(ls []*replicationv1.EventLog) *daov1.VersionVector {
	return newVersionVectorFromReplicationEventLogs(ls, versionvector.Upgrade)
}

func NewMinVersionVectorFromReplicationEventLogs(ls []*replicationv1.EventLog) *daov1.VersionVector {
	return newVersionVectorFromReplicationEventLogs(ls, versionvector.Downgrade)
}

func replicationEventLogLocalVersion(l *replicationv1.EventLog) uint64 {
	return l.Checkpoint.Version.Value[l.Checkpoint.Id]
}

func NewReplicationCheckpoint(replicaID uint64, v *daov1.VersionVector) *replicationv1.Checkpoint {
	return &replicationv1.Checkpoint{
		Id:      replicaID,
		Version: v,
	}
}

var ErrReplicatorNotFound = errors.New("replicator not found")

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

var replicators = map[namespace]Replicator{}

func NewReplicatedStore(s *ProfileStore) (*ReplicatedStore, error) {
	p, err := Profile.Get(s)
	if err != nil {
		return nil, err
	}

	rs := &ReplicatedStore{
		ProfileStore: s,
		replicaID:    p.DeviceId,
	}

	return rs, nil
}

type ReplicatedStore struct {
	*ProfileStore
	replicaID uint64
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

	c, err := ReplicationCheckpoints.Increment(tx, s.replicaID)
	if err != nil {
		return err
	}

	l := &replicationv1.EventLog{
		Id:         id,
		Checkpoint: c,
		Events:     events,
	}
	tx.(EventEmitter).Emit(l)
	return ReplicationEventLogs.Insert(tx, l)
}

func (s *ReplicatedStore) ReplicaID() uint64 {
	return s.replicaID
}

func DumpReplicationEvents(s kv.RWStore) ([]*replicationv1.Event, error) {
	var es []*replicationv1.Event
	err := s.View(func(tx kv.Tx) error {
		for _, r := range replicators {
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

func applyReplicationEvents(tx kv.RWTx, es []*replicationv1.Event) error {
	for _, e := range es {
		m, ok := replicators[namespace(e.Namespace)]
		if !ok {
			return ErrReplicatorNotFound
		}

		if err := m.ApplyEvent(tx, e); err != nil {
			return err
		}
	}
	return nil
}

func ApplyReplicationEvents(s Store, es []*replicationv1.Event, v *daov1.VersionVector) (*replicationv1.Checkpoint, error) {
	var c *replicationv1.Checkpoint
	err := s.(*ReplicatedStore).ProfileStore.Update(func(tx kv.RWTx) (err error) {
		if err := applyReplicationEvents(tx, es); err != nil {
			return err
		}
		c, err = ReplicationCheckpoints.Merge(tx, NewReplicationCheckpoint(s.ReplicaID(), v))
		return err
	})
	if err != nil {
		return nil, err
	}
	return c, nil
}

func ApplyReplicationEventLogs(s Store, ls []*replicationv1.EventLog) (*replicationv1.Checkpoint, error) {
	var c *replicationv1.Checkpoint
	err := s.(*ReplicatedStore).ProfileStore.Update(func(tx kv.RWTx) error {
		pls, err := ReplicationEventLogs.GetAllAfter(tx, NewMinVersionVectorFromReplicationEventLogs(ls))
		if err != nil {
			return err
		}

		df := newReplicationDeleteFilter(nil)
		for _, l := range pls {
			for _, e := range l.Events {
				if err := df.AddEvent(tx, e); err != nil {
					return err
				}
			}
		}

		cs := map[uint64]*replicationv1.Checkpoint{}
		f := newReplicationEventFilter(df)
		for _, l := range ls {
			for _, e := range l.Events {
				if err := f.AddEvent(tx, e); err != nil {
					return err
				}
			}

			if c, ok := cs[l.Checkpoint.Id]; !ok {
				cs[l.Checkpoint.Id] = NewReplicationCheckpoint(l.Checkpoint.Id, versionvector.New(l.Checkpoint.Version))
			} else {
				versionvector.Upgrade(c.Version, l.Checkpoint.Version)
			}
		}

		if err := applyReplicationEvents(tx, f.Events()); err != nil {
			return err
		}
		if _, err = ReplicationCheckpoints.MergeAll(tx, maps.Values(cs)); err != nil {
			return err
		}
		c, err = ReplicationCheckpoints.Merge(tx, NewReplicationCheckpoint(s.ReplicaID(), NewVersionVectorFromReplicationEventLogs(ls)))
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

type replicationEventKey struct {
	ns namespace
	id uint64
}

func newReplicationDeleteFilter(base ReplicationEventFilter) *replicationDeleteFilter {
	return &replicationDeleteFilter{
		events: map[replicationEventKey]*replicationv1.Event{},
		base:   base,
	}
}

type replicationDeleteFilter struct {
	events map[replicationEventKey]*replicationv1.Event
	base   ReplicationEventFilter
}

func (f *replicationDeleteFilter) Test(e *replicationv1.Event) bool {
	if f.base != nil && !f.base.Test(e) {
		return false
	}
	k := replicationEventKey{namespace(e.Namespace), e.Id}
	_, ok := f.events[k]
	if !ok && e.Delete {
		f.events[k] = e
		return true
	}
	return !ok
}

func (f *replicationDeleteFilter) AddEvent(s kv.Store, e *replicationv1.Event) error {
	if e.Delete {
		f.events[replicationEventKey{namespace(e.Namespace), e.Id}] = e
	}
	return nil
}

func (f *replicationDeleteFilter) Events() []*replicationv1.Event {
	return maps.Values(f.events)
}

func newReplicationEventFilter(base ReplicationEventFilter) *replciationEventFilter {
	return &replciationEventFilter{
		filters: map[namespace]ReplicationEventFilter{},
		base:    base,
	}
}

type replciationEventFilter struct {
	filters map[namespace]ReplicationEventFilter
	base    ReplicationEventFilter
}

func (f *replciationEventFilter) filter(ns namespace) (ReplicationEventFilter, error) {
	if nf, ok := f.filters[ns]; ok {
		return nf, nil
	}
	if r, ok := replicators[ns]; ok {
		nf := r.EventFilter()
		f.filters[ns] = nf
		return nf, nil
	}
	return nil, ErrReplicatorNotFound
}

func (f *replciationEventFilter) Test(e *replicationv1.Event) bool {
	if f.base != nil && !f.base.Test(e) {
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
			versionvector.Upgrade(m.GetVersion(), p.GetVersion())
			return t.Update(s, m)
		},
		Extract: func(s kv.Store, m, p T) T { return m },
		Merge: func(s kv.RWStore, m, p T) T {
			versionvector.Upgrade(m.GetVersion(), p.GetVersion())
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

	replicators[t.ns] = &TableReplicator[V, T]{
		t:   t,
		opt: opt,
	}
}

type TableReplicator[V any, T ReplicatedTableRecord[V]] struct {
	t   *Table[V, T]
	opt *ReplicatedTableOptions[T]
}

func (t *TableReplicator[V, T]) ApplyEvent(s kv.RWStore, e *replicationv1.Event) error {
	if e.Delete {
		err := t.t.Delete(s, e.Id)
		if errors.Is(err, kv.ErrRecordNotFound) {
			return nil
		}
		return err
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
		orders: map[*replicationv1.Event]int{},
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
	orders map[*replicationv1.Event]int
	order  int
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
			versionvector.Upgrade(index.Version, e.Version)
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
	t.orders[e] = t.order
	t.order++
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
	t.orders[e] = t.order
	t.order++
	return nil
}

func (t *TableReplicatorEventFilter[V, T]) Events() []*replicationv1.Event {
	es := maps.Values(t.events)
	sort.Slice(es, func(i, j int) bool { return t.orders[es[i]] < t.orders[es[j]] })
	return es
}
