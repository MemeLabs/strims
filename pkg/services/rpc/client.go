package rpc

import (
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/golang/protobuf/proto"
	"github.com/golang/protobuf/ptypes/any"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// NewClient ...
func NewClient(logger *zap.Logger, client *vpn.Client, key, salt []byte) (*Client, error) {
	port, err := client.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	c := &Client{
		logger: logger,
		client: client,
		key:    key,
		salt:   salt,
		port:   port,
		conns:  llrb.New(),
	}

	if err := c.client.Network.SetHandler(port, c); err != nil {
		return nil, err
	}
	return c, nil
}

// Client ...
type Client struct {
	logger    *zap.Logger
	client    *vpn.Client
	key       []byte
	salt      []byte
	port      uint16
	conns     *llrb.LLRB
	connsLock sync.Mutex
}

// HandleMessage ...
func (s *Client) HandleMessage(msg *vpn.Message) (bool, error) {
	m := &pb.Call{}
	if err := proto.Unmarshal(msg.Body, m); err != nil {
		return false, err
	}

	callback, ok := s.loadCallback(msg.Trailers[0].HostID, m.ParentId)
	if !ok {
		return false, nil
	}

	err := unmarshalAny(m.Argument, callback.res)
	select {
	case callback.done <- err:
	default:
	}

	return false, nil
}

func (s *Client) storeCallback(hostID kademlia.ID, callID uint64, res proto.Message) *vpnClientCallback {
	s.connsLock.Lock()
	defer s.connsLock.Unlock()

	conn, ok := s.conns.Get(&vpnClientConn{hostID: hostID}).(*vpnClientConn)
	if !ok {
		conn = &vpnClientConn{
			hostID:    hostID,
			callbacks: map[uint64]*vpnClientCallback{},
		}
		s.conns.InsertNoReplace(conn)
	}

	callback := &vpnClientCallback{
		res:  res,
		done: make(chan error),
	}
	conn.callbacks[callID] = callback
	return callback
}

func (s *Client) loadCallback(hostID kademlia.ID, callID uint64) (*vpnClientCallback, bool) {
	s.connsLock.Lock()
	defer s.connsLock.Unlock()

	conn, ok := s.conns.Get(&vpnClientConn{hostID: hostID}).(*vpnClientConn)
	if !ok {
		return nil, false
	}

	callback, ok := conn.callbacks[callID]
	return callback, ok
}

func (s *Client) discardCallback(hostID kademlia.ID, callID uint64) {
	s.connsLock.Lock()
	defer s.connsLock.Unlock()

	conn, ok := s.conns.Get(&vpnClientConn{hostID: hostID}).(*vpnClientConn)
	if !ok {
		return
	}

	delete(conn.callbacks, callID)
	if len(conn.callbacks) == 0 {
		s.conns.Delete(&vpnClientConn{hostID: hostID})
	}
}

// CallUnary ...
func (s *Client) CallUnary(ctx context.Context, method string, req, res proto.Message) error {
	ctx, cancel := context.WithTimeout(ctx, clientTimeout)
	defer cancel()

	addr, err := GetHostAddr(ctx, s.client, s.key, s.salt)
	if err != nil {
		return err
	}

	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	if err := b.Marshal(req); err != nil {
		return err
	}

	callID, err := dao.GenerateSnowflake()
	if err != nil {
		return err
	}

	cb := s.storeCallback(addr.HostID, callID, res)
	defer s.discardCallback(addr.HostID, callID)

	call := &pb.Call{
		Id:     callID,
		Method: method,
		Argument: &any.Any{
			TypeUrl: anyURLPrefix + proto.MessageName(req),
			Value:   b.Bytes(),
		},
	}
	err = s.client.Network.SendProto(addr.HostID, addr.Port, s.port, call)
	if err != nil {
		return err
	}

	select {
	case err := <-cb.done:
		return err
	case <-ctx.Done():
		return ctx.Err()
	}
}

type vpnClientConn struct {
	hostID    kademlia.ID
	port      uint16
	callbacks map[uint64]*vpnClientCallback
}

func (c *vpnClientConn) Less(o llrb.Item) bool {
	if o, ok := o.(*vpnClientConn); ok {
		return c.hostID.Less(o.hostID)
	}
	return !o.Less(c)
}

type vpnClientCallback struct {
	res  proto.Message
	done chan error
}
