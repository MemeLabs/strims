package dao

import (
	"errors"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/slab"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
)

func newCacheItemTimerWheel[V any, T TableRecord[V]](ttl, ivl time.Duration) cacheItemTimerWheel[V, T] {
	return cacheItemTimerWheel[V, T]{
		time:  timeutil.Now().Truncate(ivl),
		ttl:   ttl,
		ivl:   ivl,
		lists: make([]cacheItemList[V, T], ttl/ivl),
	}
}

type cacheItemTimerWheel[V any, T TableRecord[V]] struct {
	time  timeutil.Time
	ttl   time.Duration
	ivl   time.Duration
	lists []cacheItemList[V, T]
}

func (c *cacheItemTimerWheel[V, T]) index(t timeutil.Time) int {
	return int((t.UnixNano() / int64(c.ivl)) % int64(len(c.lists)))
}

func (c *cacheItemTimerWheel[V, T]) insert(e *cacheItem[V, T]) {
	c.lists[c.index(e.time)].insert(e)
}

func (c *cacheItemTimerWheel[V, T]) touch(e *cacheItem[V, T]) {
	e.time = c.time
}

func (c *cacheItemTimerWheel[V, T]) advance(now timeutil.Time) *cacheItem[V, T] {
	var t, l cacheItemList[V, T]

	now = now.Truncate(c.ivl)
	for c.time < now {
		c.time = c.time.Add(c.ivl)
		t.insertList(&c.lists[c.index(c.time)])
	}

	eol := c.time.Add(-c.ttl)
	for e := t.head; e != nil; {
		ce := e
		e, e.list = e.list, nil

		if eol.Before(ce.time) {
			c.insert(ce)
		} else {
			l.insert(ce)
		}
	}

	return l.head
}

type cacheItemList[V any, T TableRecord[V]] struct {
	head, tail *cacheItem[V, T]
}

func (c *cacheItemList[V, T]) insert(e *cacheItem[V, T]) {
	if c.tail != nil {
		c.tail.list = e
	}
	if c.head == nil {
		c.head = e
	}
	e.list = nil
	c.tail = e
}

func (c *cacheItemList[V, T]) insertList(o *cacheItemList[V, T]) {
	if c.head == nil {
		c.head = o.head
	}
	if c.tail != nil {
		c.tail.list = o.head
	}
	if o.tail != nil {
		c.tail = o.tail
	}
	o.head = nil
	o.tail = nil
}

type cacheItem[V any, T TableRecord[V]] struct {
	p    unsafe.Pointer
	mu   sync.Mutex
	time timeutil.Time
	list *cacheItem[V, T]
}

func (e *cacheItem[V, T]) load() T {
	return (T)(atomic.LoadPointer(&e.p))
}

func (e *cacheItem[V, T]) store(v T) {
	atomic.StorePointer(&e.p, unsafe.Pointer(v))
}

type CacheStoreOptions struct {
	TTL        time.Duration
	GCInterval time.Duration
}

var DefaultStoreOptions = &CacheStoreOptions{
	TTL:        10 * time.Minute,
	GCInterval: 1 * time.Second,
}

func newCacheStore[K, V any, T TableRecord[V]](store kv.RWStore, table *Table[V, T], opt *CacheStoreOptions) (*CacheStore[V, T], CacheAccessor[uint64, V, T]) {
	if opt == nil {
		opt = DefaultStoreOptions
	}
	s := &CacheStore[V, T]{
		store: store,
		table: table,
		alloc: slab.New[cacheItem[V, T]](),
		queue: newCacheItemTimerWheel[V, T](opt.TTL, opt.GCInterval),

		hitCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "HIT"),
		missCount: writeThroughCacheReadCount.WithLabelValues(typeName[V](), "MISS"),
		newCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "NEW"),
		errCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "ERROR"),
	}

	s.Close = timeutil.DefaultTickEmitter.Subscribe(opt.GCInterval, s.gc, nil)

	s.primaryKey = newCacheIndex((T).GetId, hashmap.NewUint64Interface[uint64])
	s.indices = append(s.indices, s.primaryKey)
	return s, CacheAccessor[uint64, V, T]{
		getByKey: table.Get,
		store:    s,
		index:    s.primaryKey,
	}
}

type CacheStore[V any, T TableRecord[V]] struct {
	store kv.RWStore
	table *Table[V, T]
	mu    sync.Mutex
	alloc *slab.Allocator[cacheItem[V, T]]
	queue cacheItemTimerWheel[V, T]

	primaryKey *cacheIndex[uint64, V, T]
	indices    []interface {
		set(v, p T, e *cacheItem[V, T])
		delete(v T)
	}

	Close timeutil.StopFunc

	hitCount  prometheus.Counter
	missCount prometheus.Counter
	newCount  prometheus.Counter
	errCount  prometheus.Counter
}

func (c *CacheStore[V, T]) gc(t timeutil.Time) {
	c.mu.Lock()
	defer c.mu.Unlock()

	for e := c.queue.advance(t); e != nil; {
		ce := e
		e = e.list

		v := ce.load()
		c.primaryKey.delete(v)
		for _, i := range c.indices {
			i.delete(v)
		}

		ce.list = nil
		ce.p = unsafe.Pointer(nil)
		c.alloc.Free(ce)
	}
}

func (c *CacheStore[V, T]) set(v, p T) *cacheItem[V, T] {
	c.mu.Lock()
	defer c.mu.Unlock()

	e, ok := c.primaryKey.get(v.GetId())
	if ok {
		e.store(v)
	} else {
		e = c.alloc.Alloc()
		e.p = unsafe.Pointer(v)
		c.touch(e)
		c.primaryKey.set(v, nil, e)
		c.queue.insert(e)
	}

	for _, i := range c.indices {
		i.set(v, p, e)
	}

	return e
}

func (c *CacheStore[V, T]) touch(e *cacheItem[V, T]) {
	c.queue.touch(e)
}

func (c *CacheStore[V, T]) get(k uint64) (e *cacheItem[V, T], err error) {
	c.mu.Lock()
	e, ok := c.primaryKey.get(k)
	c.mu.Unlock()
	if ok {
		c.hitCount.Inc()
		return e, nil
	}

	v, err := c.table.Get(c.store, k)
	if err != nil {
		c.errCount.Inc()
		return nil, err
	}

	c.missCount.Inc()
	return c.set(v, nil), nil
}

func (c *CacheStore[V, T]) Load(k uint64) error {
	_, err := c.get(k)
	return err
}

func (c *CacheStore[V, T]) Store(v T) {
	c.set(v, nil)
}

func NewCacheIndex[K, V any, T TableRecord[V]](s *CacheStore[V, T], getByKey func(store kv.Store, k K) (T, error), key func(m T) K, ifctor func() hashmap.Interface[K]) CacheAccessor[K, V, T] {
	index := newCacheIndex(key, ifctor)
	s.indices = append(s.indices, index)
	return CacheAccessor[K, V, T]{
		getByKey: getByKey,
		store:    s,
		index:    index,
	}
}

func newCacheIndex[K, V any, T TableRecord[V]](key func(m T) K, ifctor func() hashmap.Interface[K]) *cacheIndex[K, V, T] {
	iface := ifctor()
	return &cacheIndex[K, V, T]{
		key:      key,
		keyEqual: iface.Equal,
		cache:    hashmap.New[K, *cacheItem[V, T]](iface),
	}
}

type cacheIndex[K, V any, T TableRecord[V]] struct {
	key      func(m T) K
	keyEqual func(a, b K) bool
	cache    hashmap.Map[K, *cacheItem[V, T]]
}

func (c *cacheIndex[K, V, T]) get(k K) (*cacheItem[V, T], bool) {
	return c.cache.Get(k)
}

func (c *cacheIndex[K, V, T]) set(v, p T, e *cacheItem[V, T]) {
	vk := c.key(v)

	if p != nil {
		pk := c.key(p)
		if c.keyEqual(vk, pk) {
			return
		}
		c.cache.Delete(pk)
	}

	c.cache.Set(vk, e)
}

func (c *cacheIndex[K, V, T]) delete(v T) {
	c.cache.Delete(c.key(v))
}

type CacheAccessor[K, V any, T TableRecord[V]] struct {
	getByKey func(store kv.Store, k K) (T, error)
	store    *CacheStore[V, T]
	index    *cacheIndex[K, V, T]
}

func (c *CacheAccessor[K, V, T]) get(k K) (*cacheItem[V, T], bool, error) {
	c.store.mu.Lock()
	e, ok := c.index.get(k)
	if ok {
		c.store.touch(e)
	}
	c.store.mu.Unlock()
	if ok {
		c.store.hitCount.Inc()
		return e, true, nil
	}

	v, err := c.getByKey(c.store.store, k)
	if err != nil {
		c.store.errCount.Inc()
		return nil, false, err
	}

	c.store.missCount.Inc()
	return c.store.set(v, nil), false, nil
}

func (c *CacheAccessor[K, V, T]) Get(k K) (v T, err error) {
	e, _, err := c.get(k)
	if err != nil {
		return nil, err
	}
	return e.load(), nil
}

func (c *CacheAccessor[K, V, T]) GetOrInsert(k K, ctor func() (T, error)) (v T, ok bool, err error) {
	for {
		e, found, err := c.get(k)
		if err == nil {
			return e.load(), found, nil
		} else if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return nil, false, err
		}

		v, err = ctor()
		if err != nil {
			c.store.errCount.Inc()
			return nil, false, err
		}
		err = c.store.table.Insert(c.store.store, v)
		if err == nil {
			c.store.newCount.Inc()
			return c.store.set(v, nil).load(), true, nil
		} else if err != nil && !errors.Is(err, ErrUniqueConstraintViolated) {
			c.store.errCount.Inc()
			return nil, false, err
		}
	}
}

func (c *CacheAccessor[K, V, T]) Transform(k K, fn func(m T) error) (T, error) {
	e, _, err := c.get(k)
	if err != nil {
		return nil, err
	}

	e.mu.Lock()
	defer e.mu.Unlock()

	p := e.load()
	v, err := c.store.table.Transform(c.store.store, p.GetId(), fn)
	if err != nil {
		return nil, err
	}
	c.store.set(v, p)
	return v, nil
}
