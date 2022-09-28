package dao

import (
	"errors"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/debug"
	"github.com/MemeLabs/strims/pkg/event"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"go.uber.org/zap"
	"golang.org/x/exp/maps"
	"google.golang.org/protobuf/proto"
)

var ReplicationEventLogs = NewTable(
	replicationLogNS,
	&TableOptions[replicationv1.EventLog, *replicationv1.EventLog]{
		// ObserveChange: func(m, p *replicationv1.EventLog) proto.Message {
		// 	return &replicationv1.EventLog{ReplicationEventLog: m}
		// },
		// ObserveDelete: func(m *replicationv1.EventLog) proto.Message {
		// 	return &replicationv1.EventLog{ReplicationEventLog: m}
		// },
	},
)

var ReplicationVersion = NewSingleton[replicationv1.Version](replicationVersionNS, nil)

var _ ReplicatedRWTx = (*replicatedStoreTx)(nil)

type ReplicationEventFilter interface {
	AddEvent(s kv.Store, e *replicationv1.Event) error
	Events() []*replicationv1.Event
}

type Replicator interface {
	EventFilter() ReplicationEventFilter
	DispatchEvent(s kv.RWStore, e *replicationv1.Event) error
	Dump(s kv.Store) ([]*replicationv1.Event, error)
}

var replicators syncutil.Map[namespace, func() Replicator]

func NewReplicatedStore(s *ProfileStore) (*ReplicatedStore, error) {
	// panic?
	// v, err := ReplicationVersion.Get(s)
	// if err != nil {
	// 	return nil, err
	// }

	rs := &ReplicatedStore{
		ProfileStore: s,
		replicators:  map[namespace]Replicator{},
	}

	replicators.Each(func(ns namespace, ctor func() Replicator) {
		rs.replicators[ns] = ctor()
	})

	return rs, nil
}

type ReplicatedStore struct {
	*ProfileStore
	replicators map[namespace]Replicator
	observers   event.Observer
	replicaID   uint32
}

func (s *ReplicatedStore) Update(fn func(tx kv.RWTx) error) (err error) {
	txID, err := s.GenerateID()
	if err != nil {
		return err
	}

	return s.ProfileStore.Update(func(tx kv.RWTx) error {
		ptx := &replicatedStoreTx{
			profileStoreTx: tx.(*profileStoreTx),
			replicaID:      s.ReplicaID(),
		}
		if err := fn(ptx); err != nil {
			return err
		}
		if len(ptx.events) != 0 {
			return s.commitEventLog(ptx, txID, ptx.events)
		}
		return nil
	})
}

func (s *ReplicatedStore) ReplicaID() uint32 {
	return s.replicaID
}

func (s *ReplicatedStore) DispatchEvent(es []*replicationv1.Event) error {
	for _, e := range es {
		m, ok := s.replicators[namespace(e.Namespace)]
		if !ok {
			return errors.New("wur replication meme?")
		}

		if err := m.DispatchEvent(s.ProfileStore, e); err != nil {
			return err
		}
	}
	return nil
}

func (s *ReplicatedStore) commitEventLog(tx kv.RWTx, id uint64, m []*replicationv1.Event) error {
	events := make([]*replicationv1.Event, len(m))
	for i, e := range m {
		events[i] = &replicationv1.Event{
			Namespace: e.Namespace,
			Id:        e.Id,
			Version:   e.Version,
			Delete:    e.Delete,
		}
	}

	log := &replicationv1.EventLog{
		Id:        id,
		ReplicaId: s.replicaID,
		Events:    events,
	}
	if err := ReplicationEventLogs.Insert(tx, log); err != nil {
		return err
	}

	s.observers.Emit(m)
	return nil
}

func (s *ReplicatedStore) Subscribe(ch chan []*replicationv1.Event) {
	s.observers.Notify(ch)
}

func (s *ReplicatedStore) CollectEvents() error {
	f := newReplicationEventFilter(s.replicators)

	err := s.View(func(tx kv.Tx) error {
		ls, err := ReplicationEventLogs.GetAll(tx)
		if err != nil {
			return err
		}

		for _, l := range ls {
			for _, e := range l.Events {
				if err := f.AddEvent(tx, e); err != nil {
					if !errors.Is(err, errReplicatorNotFound) {
						return err
					}
					Logger.Warn("failed to replicate event", zap.Error(err))
				}
			}
		}
		return nil
	})
	if err != nil {
		return err
	}

	debug.PrintJSON(f.Events())

	return nil
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

var errReplicatorNotFound = errors.New("replicator not found")

func newReplicationEventFilter(replicators map[namespace]Replicator) *replciationEventFilter {
	return &replciationEventFilter{
		replicators: replicators,
		filters:     map[namespace]ReplicationEventFilter{},
	}
}

type replciationEventFilter struct {
	replicators map[namespace]Replicator
	filters     map[namespace]ReplicationEventFilter
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

func (f *replciationEventFilter) AddEvent(s kv.Store, e *replicationv1.Event) error {
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
	replicaID uint32
	events    []*replicationv1.Event
}

func (t *replicatedStoreTx) Update(fn func(tx kv.RWTx) error) error {
	return fn(t)
}

func (t *replicatedStoreTx) ReplicaID() uint32 {
	return t.replicaID
}

func (t *replicatedStoreTx) Replicate(e *replicationv1.Event) {
	t.events = append(t.events, e)
}

type ReplicatedRWTx interface {
	kv.RWTx
	ReplicaID() uint32
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

func (t *TableReplicator[V, T]) DispatchEvent(s kv.RWStore, e *replicationv1.Event) error {
	if e.Delete {
		return t.t.Delete(s, e.Id)
	}

	next := (T)(new(V))
	if err := proto.Unmarshal(e.Record, next); err != nil {
		return err
	}

	prev, err := t.t.Get(s, next.GetId())
	if errors.Is(err, kv.ErrRecordNotFound) {
		return t.t.Insert(s, next)
	} else if err != nil {
		return err
	}

	d, ordered := versionvector.Compare(prev.GetVersion(), next.GetVersion())
	if !ordered {
		return t.opt.OnConflict(s, next, prev)
	} else if d < 0 {
		return t.t.Update(s, t.opt.Merge(s, next, prev))
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

func (t *TableReplicatorEventFilter[V, T]) AddEvent(s kv.Store, e *replicationv1.Event) error {
	if prev, ok := t.events[e.Id]; ok {
		if prev.Delete {
			versionvector.Update(prev.Version, e.Version)
			return nil
		}

		d, ordered := versionvector.Compare(prev.Version, e.Version)
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
