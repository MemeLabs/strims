// +build !js

package videoingress

import (
	"context"

	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control/dialer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
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
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) UpdateChannel(ctx context.Context, req *video.VideoIngressShareUpdateChannelRequest) (*video.VideoIngressShareUpdateChannelResponse, error) {
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) DeleteChannel(ctx context.Context, req *video.VideoIngressShareDeleteChannelRequest) (*video.VideoIngressShareDeleteChannelResponse, error) {
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}
