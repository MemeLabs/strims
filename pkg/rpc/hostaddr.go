package rpc

import (
	"context"
	"errors"
	"fmt"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"google.golang.org/protobuf/proto"
)

// HostAddr ...
type HostAddr struct {
	HostID kademlia.ID
	Port   uint16
}

// PublishLocalHostAddr ...
func PublishLocalHostAddr(ctx context.Context, c *vpn.Client, key *pb.Key, salt []byte, port uint16) error {
	addr := &HostAddr{
		HostID: c.Host.VNIC().ID(),
		Port:   port,
	}
	return PublishHostAddr(ctx, c, key, salt, addr)
}

// PublishHostAddr ...
func PublishHostAddr(ctx context.Context, c *vpn.Client, key *pb.Key, salt []byte, addr *HostAddr) error {
	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: addr.HostID.Bytes(nil),
		Port:   uint32(addr.Port),
	})
	if err != nil {
		return err
	}

	_, err = c.HashTable.Set(ctx, key, salt, b)
	return err
}

// GetHostAddr ...
func GetHostAddr(ctx context.Context, c *vpn.Client, key, salt []byte) (*HostAddr, error) {
	values, err := c.HashTable.Get(ctx, key, salt)
	if err != nil {
		return nil, fmt.Errorf("address request failed: %w", ctx.Err())
	}

	addr := &pb.NetworkAddress{}
	select {
	case <-ctx.Done():
		return nil, fmt.Errorf("no address received: %w", ctx.Err())
	case v := <-values:
		if err := proto.Unmarshal(v, addr); err != nil {
			return nil, err
		}
	}

	hostID, err := kademlia.UnmarshalID(addr.HostId)
	if err != nil {
		return nil, fmt.Errorf("malformed address: %w", err)
	}

	if addr.Port > math.MaxUint16 {
		return nil, errors.New("port out of range")
	}

	return &HostAddr{hostID, uint16(addr.Port)}, nil
}
