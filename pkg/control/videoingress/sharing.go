// +build !js

package videoingress

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// ShareAddressSalt ...
var ShareAddressSalt = []byte("ingressshare")

func newShareService(logger *zap.Logger, vpn *vpn.Client, store *dao.ProfileStore) *shareService {
	return &shareService{}
}

type shareService struct {
}

func (s *shareService) Run(ctx context.Context) error {
	return nil
}

func (s *shareService) CreateChannel(ctx context.Context, req *pb.VideoIngressShareCreateChannelRequest) (*pb.VideoIngressShareCreateChannelResponse, error) {
	cert := rpc.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, api.ErrNotImplemented
}

func (s *shareService) UpdateChannel(ctx context.Context, req *pb.VideoIngressShareUpdateChannelRequest) (*pb.VideoIngressShareUpdateChannelResponse, error) {
	cert := rpc.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, api.ErrNotImplemented
}

func (s *shareService) DeleteChannel(ctx context.Context, req *pb.VideoIngressShareDeleteChannelRequest) (*pb.VideoIngressShareDeleteChannelResponse, error) {
	cert := rpc.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, api.ErrNotImplemented
}
