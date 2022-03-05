//go:build !js

package videoingress

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

// ShareAddressSalt ...
var ShareAddressSalt = []byte("ingressshare")

func newShareService(logger *zap.Logger, node *vpn.Node, store *dao.ProfileStore) *shareService {
	return &shareService{}
}

type shareService struct {
}

func (s *shareService) Run(ctx context.Context) error {
	return nil
}

func (s *shareService) CreateChannel(ctx context.Context, req *video.VideoIngressShareCreateChannelRequest) (*video.VideoIngressShareCreateChannelResponse, error) {
	cert := network.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) UpdateChannel(ctx context.Context, req *video.VideoIngressShareUpdateChannelRequest) (*video.VideoIngressShareUpdateChannelResponse, error) {
	cert := network.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) DeleteChannel(ctx context.Context, req *video.VideoIngressShareDeleteChannelRequest) (*video.VideoIngressShareDeleteChannelResponse, error) {
	cert := network.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}
