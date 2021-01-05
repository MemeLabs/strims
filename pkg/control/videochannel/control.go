package videochannel

import (
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/control/event"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers) *Control {
	return &Control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
}

// GetChannel ...
func (c *Control) GetChannel(id uint64) (*pb.VideoChannel, error) {
	return dao.GetVideoChannel(c.store, id)
}

// ListChannels ...
func (c *Control) ListChannels() ([]*pb.VideoChannel, error) {
	// TODO: enrich channel data with liveness?
	return dao.GetVideoChannels(c.store)
}

// CreateChannel ...
func (c *Control) CreateChannel(opts ...control.VideoChannelOption) (*pb.VideoChannel, error) {
	channel, err := dao.NewVideoChannel(c.store)
	if err != nil {
		return nil, err
	}

	if err := channelOptionSlice(opts).Apply(channel); err != nil {
		return nil, err
	}

	if err := dao.UpsertVideoChannel(c.store, channel); err != nil {
		return nil, err
	}

	return channel, err
}

// UpdateChannel ...
func (c *Control) UpdateChannel(id uint64, opts ...control.VideoChannelOption) (*pb.VideoChannel, error) {
	var channel *pb.VideoChannel
	err := c.store.Update(func(tx kv.RWTx) (err error) {
		channel, err = dao.GetVideoChannel(tx, id)
		if err != nil {
			return err
		}

		if err := channelOptionSlice(opts).Apply(channel); err != nil {
			return err
		}

		return dao.UpsertVideoChannel(c.store, channel)
	})
	if err != nil {
		return nil, err
	}

	c.observers.EmitGlobal(event.VideoChannelUpdate{Channel: channel})

	return channel, err
}

type channelOptionSlice []control.VideoChannelOption

func (s channelOptionSlice) Apply(channel *pb.VideoChannel) error {
	for _, o := range s {
		if err := o(channel); err != nil {
			return err
		}
	}
	return nil
}

// WithDirectorySnippet ...
func WithDirectorySnippet(snippet *pb.DirectoryListingSnippet) control.VideoChannelOption {
	return func(channel *pb.VideoChannel) error {
		channel.DirectoryListingSnippet = snippet
		return nil
	}
}

// WithLocalOwner ...
func WithLocalOwner(profileKey, networkKey []byte) control.VideoChannelOption {
	return func(channel *pb.VideoChannel) error {
		channel.Owner = &pb.VideoChannel_Local_{
			Local: &pb.VideoChannel_Local{
				AuthKey:    profileKey,
				NetworkKey: networkKey,
			},
		}
		return nil
	}
}

// WithLocalShareOwner ...
func WithLocalShareOwner(cert *pb.Certificate) control.VideoChannelOption {
	return func(channel *pb.VideoChannel) error {
		channel.Owner = &pb.VideoChannel_LocalShare_{
			LocalShare: &pb.VideoChannel_LocalShare{
				Certificate: cert,
			},
		}
		return nil
	}
}

// WithRemoteShareOwner ...
func WithRemoteShareOwner(share *pb.VideoChannel_RemoteShare) control.VideoChannelOption {
	return func(channel *pb.VideoChannel) error {
		channel.Owner = &pb.VideoChannel_RemoteShare_{
			RemoteShare: share,
		}
		return nil
	}
}

// DeleteChannel ...
func (c *Control) DeleteChannel(id uint64) error {
	if err := dao.DeleteVideoChannel(c.store, id); err != nil {
		return err
	}

	c.observers.EmitGlobal(event.VideoChannelRemove{ID: id})

	return nil
}
