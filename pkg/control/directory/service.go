package directory

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

const broadcastInterval = time.Second
const sessionTimeout = time.Minute * 15

// AddressSalt ...
var AddressSalt = []byte("directory")

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

func newDirectoryService(logger *zap.Logger, key *pb.Key) (*directoryService, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:  128,
			LiveWindow: 16 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod: integrity.ProtectionMethodSignAll,
			},
		},
		Key: key,
	})
	if err != nil {
		return nil, err
	}

	return &directoryService{
		logger:          logger,
		done:            make(chan struct{}),
		broadcastTicker: time.NewTicker(broadcastInterval),
		w:               prefixstream.NewWriter(w),
		swarm:           w.Swarm(),
	}, nil
}

type directoryService struct {
	logger            *zap.Logger
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   *time.Ticker
	lastBroadcastTime time.Time
	w                 *prefixstream.Writer
	swarm             *ppspp.Swarm
	lock              sync.Mutex
	listings          lru
	sessions          lru
	users             lru
	certificate       *pb.Certificate
}

func (d *directoryService) Run(ctx context.Context) error {
	for {
		select {
		case <-d.broadcastTicker.C:
			if err := d.broadcast(); err != nil {
				return err
			}
		case <-d.done:
			return errors.New("closed")
		case <-ctx.Done():
			d.Close()
			return ctx.Err()
		}
	}
}

func (d *directoryService) Close() {
	d.closeOnce.Do(func() {
		// TODO: shut down swarm...
		d.broadcastTicker.Stop()
		close(d.done)
	})
}

func (d *directoryService) broadcast() error {
	d.lock.Lock()
	defer d.lock.Unlock()

	now := iotime.Load()

	for it := d.listings.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
		l := it.Value().(*listing)

		var event *pb.DirectoryServerEvent
		if l.publishers.Len() == 0 && l.viewers.Len() == 0 {
			d.listings.Delete(l)
			event = &pb.DirectoryServerEvent{
				Body: &pb.DirectoryServerEvent_Unpublish_{
					Unpublish: &pb.DirectoryServerEvent_Unpublish{
						Key: l.listing.Key,
					},
				},
			}
		} else if l.modifiedTime.After(d.lastBroadcastTime) {
			event = &pb.DirectoryServerEvent{
				Body: &pb.DirectoryServerEvent_Publish_{
					Publish: &pb.DirectoryServerEvent_Publish{
						Listing: l.listing,
					},
				},
			}
		} else {
			event = &pb.DirectoryServerEvent{
				Body: &pb.DirectoryServerEvent_ViewerCountChange_{
					ViewerCountChange: &pb.DirectoryServerEvent_ViewerCountChange{
						Key:   l.listing.Key,
						Count: uint32(l.viewers.Len()),
					},
				},
			}
		}
		if err := d.writeToStream(event); err != nil {
			return err
		}
	}

	for it := d.users.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
		u := it.Value().(*user)

		var keys [][]byte
		u.sessions.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			it.(*session).viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
				keys = append(keys, it.(*listing).listing.Key)
				return true
			})
			return true
		})

		event := &pb.DirectoryServerEvent{
			Body: &pb.DirectoryServerEvent_ViewerStateChange_{
				ViewerStateChange: &pb.DirectoryServerEvent_ViewerStateChange{
					Subject:     u.certificate.Subject,
					Online:      true,
					ViewingKeys: keys,
				},
			},
		}
		if err := d.writeToStream(event); err != nil {
			return err
		}
	}

	eol := now.Add(-sessionTimeout)

	for s, ok := d.sessions.Pop(eol).(*session); ok; s, ok = d.sessions.Pop(eol).(*session) {
		u := d.users.GetOrInsert(&user{certificate: s.certificate.GetParent()}).(*user)
		u.sessions.Delete(s)
		if u.sessions.Len() != 0 {
			continue
		}

		d.users.Delete(u)

		event := &pb.DirectoryServerEvent{
			Body: &pb.DirectoryServerEvent_ViewerStateChange_{
				ViewerStateChange: &pb.DirectoryServerEvent_ViewerStateChange{
					Subject: u.certificate.Subject,
					Online:  false,
				},
			},
		}
		if err := d.writeToStream(event); err != nil {
			return err
		}
	}

	for l, ok := d.listings.Pop(eol).(*listing); ok; l, ok = d.listings.Pop(eol).(*listing) {
		event := &pb.DirectoryServerEvent{
			Body: &pb.DirectoryServerEvent_Unpublish_{
				Unpublish: &pb.DirectoryServerEvent_Unpublish{
					Key: l.listing.Key,
				},
			},
		}
		if err := d.writeToStream(event); err != nil {
			return err
		}
	}

	d.lastBroadcastTime = now
	return nil
}

func (d *directoryService) writeToStream(msg protoreflect.ProtoMessage) error {
	b := pool.Get(uint16(proto.Size(msg)))
	defer pool.Put(b)

	var err error
	*b, err = proto.MarshalOptions{}.MarshalAppend((*b)[:0], msg)
	if err != nil {
		return err
	}

	_, err = d.w.Write(*b)
	return err
}

func (d *directoryService) Publish(ctx context.Context, req *pb.DirectoryPublishRequest) (*pb.DirectoryPublishResponse, error) {
	if err := dao.VerifyMessage(req.Listing); err != nil {
		return nil, err
	}

	d.lock.Lock()
	defer d.lock.Unlock()

	l := d.listings.GetOrInsert(&listing{
		listing:      req.Listing,
		modifiedTime: iotime.Load(),
	}).(*listing)
	s := d.sessions.GetOrInsert(&session{certificate: rpc.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: rpc.VPNCertificate(ctx).GetParent()}).(*user)

	if req.Listing.Timestamp > l.listing.Timestamp {
		l.listing = req.Listing
		l.modifiedTime = iotime.Load()
	}

	l.publishers.InsertNoReplace(s)
	s.publishedListings.InsertNoReplace(l)
	u.sessions.InsertNoReplace(s)

	return &pb.DirectoryPublishResponse{}, nil
}

func (d *directoryService) Unpublish(ctx context.Context, req *pb.DirectoryUnpublishRequest) (*pb.DirectoryUnpublishResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.Get(&lruKey{key: req.Key}).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s, ok := d.sessions.Get(&session{certificate: rpc.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}

	if l.publishers.Delete(s) == nil {
		return nil, errors.New("not publishing this listing")
	}
	s.publishedListings.Delete(l)

	return &pb.DirectoryUnpublishResponse{}, nil
}

func (d *directoryService) Join(ctx context.Context, req *pb.DirectoryJoinRequest) (*pb.DirectoryJoinResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.Get(&lruKey{key: req.Key}).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s := d.sessions.GetOrInsert(&session{certificate: rpc.VPNCertificate(ctx)}).(*session)
	u := d.users.GetOrInsert(&user{certificate: rpc.VPNCertificate(ctx).GetParent()}).(*user)

	l.viewers.InsertNoReplace(s)
	s.viewedListings.InsertNoReplace(l)
	u.sessions.InsertNoReplace(s)

	return &pb.DirectoryJoinResponse{}, nil
}

func (d *directoryService) Part(ctx context.Context, req *pb.DirectoryPartRequest) (*pb.DirectoryPartResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	l, ok := d.listings.Get(&lruKey{key: req.Key}).(*listing)
	if !ok {
		return nil, ErrListingNotFound
	}
	s, ok := d.sessions.Get(&session{certificate: rpc.VPNCertificate(ctx)}).(*session)
	if !ok {
		return nil, ErrSessionNotFound
	}
	d.users.Touch(&user{certificate: rpc.VPNCertificate(ctx).GetParent()})

	if l.viewers.Delete(s) == nil {
		return nil, errors.New("not viewing this listing")
	}
	s.viewedListings.Delete(l)

	return &pb.DirectoryPartResponse{}, nil
}

func (d *directoryService) Ping(ctx context.Context, req *pb.DirectoryPingRequest) (*pb.DirectoryPingResponse, error) {
	d.lock.Lock()
	defer d.lock.Unlock()

	s, ok := d.sessions.Get(&session{certificate: rpc.VPNCertificate(ctx)}).(*session)
	if ok {
		return nil, ErrSessionNotFound
	}

	s.publishedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		d.listings.Touch(it.(*listing))
		return true
	})
	s.viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
		d.listings.Touch(it.(*listing))
		return true
	})

	return &pb.DirectoryPingResponse{}, nil
}

type listing struct {
	listing      *pb.DirectoryListing
	modifiedTime time.Time
	publishers   llrb.LLRB
	viewers      llrb.LLRB
}

func (l *listing) Key() []byte {
	return l.listing.Key
}

func (l *listing) Less(o llrb.Item) bool {
	return keyerLess(l, o)
}

type session struct {
	certificate       *pb.Certificate
	publishedListings llrb.LLRB
	viewedListings    llrb.LLRB
}

func (u *session) Key() []byte {
	return u.certificate.Key
}

func (u *session) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}

type user struct {
	certificate *pb.Certificate
	sessions    llrb.LLRB
}

func (u *user) Key() []byte {
	return u.certificate.Key
}

func (u *user) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}
