package dialer

import (
	"context"
	"errors"
	"fmt"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	vpnv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vpn/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"google.golang.org/protobuf/proto"
)

// HostAddr ...
type HostAddr struct {
	HostID kademlia.ID
	Port   uint16
}

type HostAddrPublisher interface {
	Publish(ctx context.Context, node *vpn.Node, addr *HostAddr) error
}

type DHTHostAddrPublisher struct {
	Key  *key.Key
	Salt []byte
}

// Publish ...
func (p *DHTHostAddrPublisher) Publish(ctx context.Context, node *vpn.Node, addr *HostAddr) error {
	b, err := proto.Marshal(&vpnv1.NetworkAddress{
		HostId: addr.HostID.Bytes(nil),
		Port:   uint32(addr.Port),
	})
	if err != nil {
		return err
	}

	_, err = node.HashTable.Set(ctx, p.Key, p.Salt, b)
	return err
}

type HostAddrResolver interface {
	Resolve(ctx context.Context, node *vpn.Node) (*HostAddr, error)
}

type DHTHostAddrResolver struct {
	Key  []byte
	Salt []byte
}

// Resolve ...
func (r *DHTHostAddrResolver) Resolve(ctx context.Context, node *vpn.Node) (*HostAddr, error) {
	values, err := node.HashTable.Get(ctx, r.Key, r.Salt)
	if err != nil {
		return nil, fmt.Errorf("address request failed: %w", ctx.Err())
	}

	addr := &vpnv1.NetworkAddress{}
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

type StaticHostAddrResolver struct {
	HostAddr
}

func (r *StaticHostAddrResolver) Resolve(ctx context.Context, node *vpn.Node) (*HostAddr, error) {
	return &r.HostAddr, nil
}
