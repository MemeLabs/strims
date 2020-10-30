package network

import (
	"context"
	"errors"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var caAddrSalt = []byte("ca:addr")
var caNetworkPort = 14

// NewCA ...
func NewCA(ctx context.Context, logger *zap.Logger, client *vpn.Client, key *pb.Key) (*CA, error) {
	ctx, cancel := context.WithCancel(ctx)
	ca := &CA{
		ctx:    ctx,
		cancel: cancel,
		logger: logger,
		key:    key,
		client: client,
	}

	var err error
	ca.port, err = client.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	go ca.stop()

	if err := ca.run(); err != nil {
		cancel()
	}

	return ca, nil
}

// CA ...
type CA struct {
	ctx    context.Context
	cancel context.CancelFunc
	logger *zap.Logger
	key    *pb.Key
	client *vpn.Client
	port   uint16
	// invite policy
	// certificate revocation stream
	// certificate transparency list?
}

func (s *CA) run() error {
	err := PublishLocalHostAddr(s.ctx, s.client, s.key, caAddrSalt, s.port)
	if err != nil {
		return err
	}

	return s.client.Network.SetHandler(s.port, s)
}

func (s *CA) stop() {
	<-s.ctx.Done()
	s.client.Network.RemoveHandler(s.port)
	s.client.Network.ReleasePort(s.port)
}

// HandleMessage ...
func (s *CA) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	var m pb.CAMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	var resp proto.Message

	switch b := m.Body.(type) {
	case *pb.CAMessage_UpgradeRequest_:
		resp, err = s.handleUpgradeRequest(b.UpgradeRequest)
	default:
		err = errors.New("unexpected message type")
	}

	if err != nil {
		s.logger.Error("failed to handle nickerv message",
			// zap.Uint64("requestID", m.RequestId),
			zap.String("requestType", reflect.TypeOf(m.Body).Name()),
			zap.Error(err),
		)

		resp = &pb.CAMessage{
			Body: &pb.CAMessage_Error{
				Error: err.Error(),
			},
		}
	}

	// TODO: return some errors that can occur during handling

	return false, s.send(resp, msg.Trailers[0].HostID, msg.Header.SrcPort, msg.Header.DstPort)
}

func (s *CA) send(msg proto.Message, dstID kademlia.ID, dstPort, srcPort uint16) error {
	return s.client.Network.SendProto(dstID, dstPort, srcPort, msg)
}

func (s *CA) handleUpgradeRequest(req *pb.CAMessage_UpgradeRequest) (*pb.CAMessage, error) {
	return nil, nil
}

// NewCAClient ...
func NewCAClient(logger *zap.Logger, client *vpn.Client) *CAClient {
	return &CAClient{
		logger: logger,
		client: client,
	}
}

// CAClient ...
type CAClient struct {
	logger *zap.Logger
	client *vpn.Client
	key    []byte
	port   uint16
}

func (s *CAClient) send(ctx context.Context, msg proto.Message) error {
	addr, err := GetHostAddr(ctx, s.client, s.key, caAddrSalt)
	if err != nil {
		return err
	}

	return s.client.Network.SendProto(addr.HostID, addr.Port, s.port, msg)
}

// RequestUpgrade ...
func (s *CAClient) RequestUpgrade() {

}
