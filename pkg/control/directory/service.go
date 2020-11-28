package directory

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

func newDirectoryService(logger *zap.Logger, client *vpn.Client, key *pb.Key) (*directoryService, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:  128,
			LiveWindow: 1024 * 1024,
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
		logger: logger,
		w:      prefixstream.NewWriter(w),
		swarm:  w.Swarm(),
	}, nil
}

type directoryService struct {
	logger      *zap.Logger
	w           *prefixstream.Writer
	swarm       *ppspp.Swarm
	lock        sync.Mutex
	listings    llrb.LLRB
	users       llrb.LLRB
	certificate *pb.Certificate
}

func (s *directoryService) writeToStream(msg protoreflect.ProtoMessage) error {
	b := pool.Get(uint16(proto.Size(msg)))
	defer pool.Put(b)

	var err error
	*b, err = proto.MarshalOptions{}.MarshalAppend((*b)[:0], msg)
	if err != nil {
		return err
	}

	_, err = s.w.Write(*b)
	return err
}

func (s *directoryService) verifyMessage(msg *vpn.Message, hostCert *pb.Certificate) error {
	identCert := hostCert.GetParent()
	networkCert := identCert.GetParent()

	if !bytes.Equal(hostCert.GetKey(), msg.Trailer.Entries[0].HostID.Bytes(nil)) {
		return errors.New("certificate host id mismatch")
	}
	if !bytes.Equal(dao.GetRootCert(s.certificate).Key, networkCert.GetKey()) {
		return errors.New("network key mismatch")
	}
	if err := dao.VerifyCertificate(hostCert); err != nil {
		return err
	}
	if !msg.Verify(0) {
		return errors.New("invalid message signature")
	}
	return nil
}

func (s *directoryService) Publish(ctx context.Context, req *pb.DirectoryPublishRequest) (*pb.DirectoryPublishResponse, error) {
	if err := s.verifyMessage(ctx.(*rpc.VPNContext).Message(), req.Certificate); err != nil {
		return nil, err
	}

	s.lock.Lock()
	defer s.lock.Unlock()

	s.listings.Get(&listing{listing: req.Listing})
	s.users.Get(&user{})

	// is this new? is it a duplicate?

	if err := s.writeToStream(req.Listing); err != nil {
		return nil, err
	}

	return nil, errors.New("not implemented")
}

func (s *directoryService) Unpublish(ctx context.Context, req *pb.DirectoryUnpublishRequest) (*pb.DirectoryUnpublishResponse, error) {
	return nil, errors.New("not implemented")
}

func (s *directoryService) Join(ctx context.Context, req *pb.DirectoryJoinRequest) (*pb.DirectoryJoinResponse, error) {
	// hostCert := req.GetCertificate()
	// identCert := hostCert.GetParent()
	// networkCert := identCert.GetParent()

	// if !bytes.Equal(dao.GetRootCert(s.certificate).Key, req.Certificate.GetParent().GetKey()) {
	// 	return nil, errors.New("network key mismatch")
	// }

	s.lock.Lock()
	defer s.lock.Unlock()

	return nil, errors.New("not implemented")
}

func (s *directoryService) Part(ctx context.Context, req *pb.DirectoryPartRequest) (*pb.DirectoryPartResponse, error) {
	return nil, errors.New("not implemented")
}

func (s *directoryService) Ping(ctx context.Context, req *pb.DirectoryPingRequest) (*pb.DirectoryPingResponse, error) {
	return nil, errors.New("not implemented")
}

type listing struct {
	listing    *pb.DirectoryListing
	users      llrb.LLRB
	publishers llrb.LLRB
}

func (i *listing) Less(o llrb.Item) bool {
	if o, ok := o.(*listing); ok {
		return bytes.Compare(i.listing.Key, o.listing.Key) == -1
	}
	return !o.Less(i)
}

type user struct {
	certificate *pb.Certificate
	listings    llrb.LLRB
}

func (i *user) Less(o llrb.Item) bool {
	if o, ok := o.(*user); ok {
		return bytes.Compare(i.certificate.Key, o.certificate.Key) == -1
	}
	return !o.Less(i)
}
