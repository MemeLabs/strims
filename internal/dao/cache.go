package dao

import (
	"errors"
	"math"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
)

type cacheItemKind int

var cacheItemKindFound cacheItemKind = -1

type cacheItem[V any, T TableRecord[V]] struct {
	p          unsafe.Pointer
	i          cacheItemKind
	mu         sync.Mutex
	time       timeutil.Time
	next, prev *cacheItem[V, T]
}

func (e *cacheItem[V, T]) reset() {
	e.next = nil
	e.prev = nil
	e.p = nil
}

func (e *cacheItem[V, T]) load() T {
	if e.i != cacheItemKindFound {
		panic("invalid access of negative cache entry")
	}
	return (T)(atomic.LoadPointer(&e.p))
}

func (e *cacheItem[V, T]) store(v T) {
	e.i = cacheItemKindFound
	atomic.StorePointer(&e.p, unsafe.Pointer(v))
}

func (e *cacheItem[V, T]) swap(v T) T {
	return (T)(atomic.SwapPointer(&e.p, unsafe.Pointer(v)))
}

func (e *cacheItem[V, T]) notFound() bool {
	return e.i != cacheItemKindFound
}

func (e *cacheItem[V, T]) storeNotFound(i int, k unsafe.Pointer) {
	e.i = cacheItemKind(i)
	e.p = k
}

func (e *cacheItem[V, T]) loadNotFound() (int, unsafe.Pointer) {
	if e.i == cacheItemKindFound {
		panic("invalid access of positive cache entry")
	}
	return int(e.i), e.p
}

type cacheItemList[V any, T TableRecord[V]] struct {
	head, tail *cacheItem[V, T]
}

func (c *cacheItemList[V, T]) delete(e *cacheItem[V, T]) {
	if c.tail == e {
		c.tail = e.prev
	}
	if c.head == e {
		c.head = e.next
	}
	if e.prev != nil {
		e.prev.next = e.next
	}
	if e.next != nil {
		e.next.prev = e.prev
	}
}

func (c *cacheItemList[V, T]) push(e *cacheItem[V, T]) {
	e.next = c.head
	e.prev = nil

	if e.next != nil {
		e.next.prev = e
	}

	c.head = e
	if c.tail == nil {
		c.tail = e
	}
}

func (c *cacheItemList[V, T]) pop() *cacheItem[V, T] {
	e := c.tail
	if e != nil {
		c.delete(e)
	}
	return e
}

func (c *cacheItemList[V, T]) peek() *cacheItem[V, T] {
	return c.tail
}

type CacheStoreOptions struct {
	TTL        time.Duration
	GCInterval time.Duration
	Cap        int
}

func (o *CacheStoreOptions) Assign(u *CacheStoreOptions) {
	if u.TTL != 0 {
		o.TTL = u.TTL
	}
	if u.GCInterval != 0 {
		o.GCInterval = u.GCInterval
	}
	if u.Cap != 0 {
		o.Cap = u.Cap
	}
}

var DefaultStoreOptions = CacheStoreOptions{
	TTL:        10 * time.Minute,
	GCInterval: 1 * time.Second,
	Cap:        math.MaxInt,
}

func newCacheStore[K, V any, T TableRecord[V]](store kv.RWStore, table *Table[V, T], opt *CacheStoreOptions) (*CacheStore[V, T], CacheAccessor[uint64, V, T]) {
	o := DefaultStoreOptions
	if opt != nil {
		o.Assign(opt)
	}

	s := &CacheStore[V, T]{
		store: store,
		table: table,

		time: timeutil.Now(),
		ttl:  o.TTL,
		cap:  o.Cap,

		hitCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "HIT"),
		missCount: writeThroughCacheReadCount.WithLabelValues(typeName[V](), "MISS"),
		newCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "NEW"),
		errCount:  writeThroughCacheReadCount.WithLabelValues(typeName[V](), "ERROR"),
	}

	s.Close = timeutil.DefaultTickEmitter.Subscribe(o.GCInterval, s.gc, nil)

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

	list cacheItemList[V, T]
	time timeutil.Time
	ttl  time.Duration
	cap  int

	primaryKey *cacheIndex[uint64, V, T]
	indices    []interface {
		set(v, p T, e *cacheItem[V, T]) *cacheItem[V, T]
		delete(v T)
		getOrSetNotFound(k unsafe.Pointer, e *cacheItem[V, T]) (*cacheItem[V, T], bool)
		deleteNotFound(k unsafe.Pointer)
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

	c.time = t

	eol := t.Add(-c.ttl)
	for {
		e := c.list.peek()
		if e == nil || eol.Before(e.time) {
			return
		}
		c.pop()
	}
}

func (c *CacheStore[V, T]) pop() {
	e := c.list.pop()

	if e.notFound() {
		i, k := e.loadNotFound()
		c.indices[i].deleteNotFound(k)
	} else {
		v := e.load()
		c.primaryKey.delete(v)
		for _, i := range c.indices {
			i.delete(v)
		}
	}

	e.reset()
}

func (c *CacheStore[V, T]) touch(e *cacheItem[V, T]) {
	if e != nil {
		e.time = c.time
		c.list.delete(e)
		c.list.push(e)
	}
}

func (c *CacheStore[V, T]) set(v T) *cacheItem[V, T] {
	c.mu.Lock()
	defer c.mu.Unlock()

	var p T
	e, ok := c.primaryKey.get(v.GetId())
	if !ok {
		if c.primaryKey.len() >= c.cap {
			c.pop()
		}

		e = &cacheItem[V, T]{}
		e.store(v)
	} else {
		p = e.swap(v)
	}
	c.touch(e)

	for _, i := range c.indices {
		if pe := i.set(v, p, e); pe != nil {
			c.list.delete(pe)
			pe.reset()
		}
	}

	return e
}

func (c *CacheStore[V, T]) setNotFound(i int, k unsafe.Pointer) *cacheItem[V, T] {
	c.mu.Lock()
	defer c.mu.Unlock()

	e := &cacheItem[V, T]{}
	if pe, ok := c.indices[i].getOrSetNotFound(k, e); ok {

		return pe
	}

	e.storeNotFound(i, k)
	c.touch(e)

	return e
}

func (c *CacheStore[V, T]) Store(v T) {
	c.set(v)
}

func (c *CacheStore[V, T]) Len() int {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.primaryKey.len()
}

func NewCacheIndex[K, V any, T TableRecord[V]](s *CacheStore[V, T], getByKey func(store kv.Store, k K) (T, error), key func(m T) K, ifctor func() hashmap.Interface[K]) CacheAccessor[K, V, T] {
	index := newCacheIndex(key, ifctor)
	s.indices = append(s.indices, index)
	return CacheAccessor[K, V, T]{
		i:        len(s.indices) - 1,
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

func (c *cacheIndex[K, V, T]) len() int {
	return c.cache.Len()
}

func (c *cacheIndex[K, V, T]) get(k K) (*cacheItem[V, T], bool) {
	return c.cache.Get(k)
}

func (c *cacheIndex[K, V, T]) set(v, p T, e *cacheItem[V, T]) *cacheItem[V, T] {
	vk := c.key(v)

	if p != nil {
		pk := c.key(p)
		if c.keyEqual(vk, pk) {
			return nil
		}
		c.cache.Delete(pk)
	}

	return c.cache.Swap(vk, e)
}

func (c *cacheIndex[K, V, T]) getOrSetNotFound(k unsafe.Pointer, e *cacheItem[V, T]) (*cacheItem[V, T], bool) {
	if pe, ok := c.cache.Get(*(*K)(k)); ok {
		return pe, true
	}
	c.cache.Set(*(*K)(k), e)
	return e, false
}

func (c *cacheIndex[K, V, T]) delete(v T) {
	c.cache.Delete(c.key(v))
}

func (c *cacheIndex[K, V, T]) deleteNotFound(k unsafe.Pointer) {
	c.cache.Delete(*(*K)(k))
}

type CacheAccessor[K, V any, T TableRecord[V]] struct {
	i        int
	getByKey func(store kv.Store, k K) (T, error)
	store    *CacheStore[V, T]
	index    *cacheIndex[K, V, T]
}

func (c *CacheAccessor[K, V, T]) get(k K) (*cacheItem[V, T], bool, error) {
	c.store.mu.Lock()
	e, ok := c.index.get(k)
	c.store.touch(e)
	c.store.mu.Unlock()
	if ok {
		c.store.hitCount.Inc()
		if e.notFound() {
			return nil, true, kv.ErrRecordNotFound
		}
		return e, true, nil
	}

	v, err := c.getByKey(c.store.store, k)
	if err == nil {
		c.store.missCount.Inc()
		return c.store.set(v), false, nil
	} else if errors.Is(err, kv.ErrRecordNotFound) {
		c.store.missCount.Inc()
		return c.store.setNotFound(c.i, unsafe.Pointer(&k)), false, err
	}

	c.store.errCount.Inc()
	return nil, false, err
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
		} else if !errors.Is(err, kv.ErrRecordNotFound) {
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
			return c.store.set(v).load(), false, nil
		} else if !errors.Is(err, ErrUniqueConstraintViolated) {
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

	v, err := c.store.table.Transform(c.store.store, e.load().GetId(), fn)
	if err != nil {
		return nil, err
	}
	c.store.set(v)
	return v, nil
}
