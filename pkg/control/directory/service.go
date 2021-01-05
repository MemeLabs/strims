package directory

import (
	"context"
	"errors"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

const broadcastInterval = time.Second
const sessionTimeout = time.Minute * 15
const pingInterval = time.Minute * 15

// AddressSalt ...
var AddressSalt = []byte("directory")

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

const chunkSize = 128

var swarmOptions = ppspp.SwarmOptions{
	ChunkSize:  chunkSize,
	LiveWindow: 16 * 1024,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod: integrity.ProtectionMethodSignAll,
	},
}

func newDirectoryService(logger *zap.Logger, key *pb.Key, transfer *transfer.Control) (*directoryService, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: swarmOptions,
		Key:          key,
	})
	if err != nil {
		return nil, err
	}

	transferID := transfer.Add(w.Swarm(), AddressSalt)
	transfer.Publish(transferID, key.Public)

	return &directoryService{
		logger:          logger,
		transfer:        transfer,
		done:            make(chan struct{}),
		broadcastTicker: time.NewTicker(broadcastInterval),
		eventWriter:     newEventWriter(w),
		swarm:           w.Swarm(),
		transferID:      transferID,
		eventReader:     newEventReader(w.Swarm().Reader()),
	}, nil
}

type directoryService struct {
	logger            *zap.Logger
	transfer          *transfer.Control
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   *time.Ticker
	lastBroadcastTime time.Time
	eventWriter       *eventWriter
	swarm             *ppspp.Swarm
	transferID        []byte
	lock              sync.Mutex
	listings          lru
	sessions          lru
	users             lru
	certificate       *pb.Certificate
	*eventReader
}

func (d *directoryService) Run(ctx context.Context) error {
	defer d.Close()

	for {
		select {
		case now := <-d.broadcastTicker.C:
			if err := d.broadcast(now); err != nil {
				return err
			}
		case <-d.done:
			return errors.New("closed")
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func (d *directoryService) Close() {
	d.closeOnce.Do(func() {
		d.transfer.Remove(d.transferID)
		d.broadcastTicker.Stop()
		close(d.done)
	})
}

func (d *directoryService) broadcast(now time.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	for it := d.listings.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
		log.Println("recently added listing")
		l := it.Value().(*listing)

		var event *pb.DirectoryEvent
		if l.publishers.Len() == 0 && l.viewers.Len() == 0 {
			d.listings.Delete(l)
			event = &pb.DirectoryEvent{
				Body: &pb.DirectoryEvent_Unpublish_{
					Unpublish: &pb.DirectoryEvent_Unpublish{
						Key: l.listing.Key,
					},
				},
			}
		} else if l.modifiedTime.After(d.lastBroadcastTime) {
			event = &pb.DirectoryEvent{
				Body: &pb.DirectoryEvent_Publish_{
					Publish: &pb.DirectoryEvent_Publish{
						Listing: l.listing,
					},
				},
			}
		} else {
			event = &pb.DirectoryEvent{
				Body: &pb.DirectoryEvent_ViewerCountChange_{
					ViewerCountChange: &pb.DirectoryEvent_ViewerCountChange{
						Key:   l.listing.Key,
						Count: uint32(l.viewers.Len()),
					},
				},
			}
		}
		if err := d.eventWriter.Write(event); err != nil {
			return err
		}
	}

	for it := d.users.PeekRecentlyTouched(d.lastBroadcastTime); it.Next(); {
		log.Println("recently added user")
		u := it.Value().(*user)

		var keys [][]byte
		u.sessions.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			it.(*session).viewedListings.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
				keys = append(keys, it.(*listing).listing.Key)
				return true
			})
			return true
		})

		event := &pb.DirectoryEvent{
			Body: &pb.DirectoryEvent_ViewerStateChange_{
				ViewerStateChange: &pb.DirectoryEvent_ViewerStateChange{
					Subject:     u.certificate.Subject,
					Online:      true,
					ViewingKeys: keys,
				},
			},
		}
		if err := d.eventWriter.Write(event); err != nil {
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

		event := &pb.DirectoryEvent{
			Body: &pb.DirectoryEvent_ViewerStateChange_{
				ViewerStateChange: &pb.DirectoryEvent_ViewerStateChange{
					Subject: u.certificate.Subject,
					Online:  false,
				},
			},
		}
		if err := d.eventWriter.Write(event); err != nil {
			return err
		}
	}

	for l, ok := d.listings.Pop(eol).(*listing); ok; l, ok = d.listings.Pop(eol).(*listing) {
		event := &pb.DirectoryEvent{
			Body: &pb.DirectoryEvent_Unpublish_{
				Unpublish: &pb.DirectoryEvent_Unpublish{
					Key: l.listing.Key,
				},
			},
		}
		if err := d.eventWriter.Write(event); err != nil {
			return err
		}
	}

	event := &pb.DirectoryEvent{
		Body: &pb.DirectoryEvent_Ping_{
			Ping: &pb.DirectoryEvent_Ping{
				Time: now.Unix(),
			},
		},
	}
	if err := d.eventWriter.Write(event); err != nil {
		return err
	}

	d.lastBroadcastTime = now
	return nil
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
