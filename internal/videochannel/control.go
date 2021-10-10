package videochannel

import (
	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

var _ control.VideoChannelControl = &Control{}

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
func (c *Control) GetChannel(id uint64) (*video.VideoChannel, error) {
	return dao.GetVideoChannel(c.store, id)
}

// ListChannels ...
func (c *Control) ListChannels() ([]*video.VideoChannel, error) {
	// TODO: enrich channel data with liveness?
	return dao.GetVideoChannels(c.store)
}

// CreateChannel ...
func (c *Control) CreateChannel(opts ...control.VideoChannelOption) (*video.VideoChannel, error) {
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
func (c *Control) UpdateChannel(id uint64, opts ...control.VideoChannelOption) (*video.VideoChannel, error) {
	var channel *video.VideoChannel
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

func (s channelOptionSlice) Apply(channel *video.VideoChannel) error {
	for _, o := range s {
		if err := o(channel); err != nil {
			return err
		}
	}
	return nil
}

// WithDirectorySnippet ...
func WithDirectorySnippet(snippet *networkv1directory.ListingSnippet) control.VideoChannelOption {
	return func(channel *video.VideoChannel) error {
		channel.DirectoryListingSnippet = snippet
		return nil
	}
}

// WithLocalOwner ...
func WithLocalOwner(profileKey, networkKey []byte) control.VideoChannelOption {
	return func(channel *video.VideoChannel) error {
		channel.Owner = &video.VideoChannel_Local_{
			Local: &video.VideoChannel_Local{
				AuthKey:    profileKey,
				NetworkKey: networkKey,
			},
		}
		return nil
	}
}

// WithLocalShareOwner ...
func WithLocalShareOwner(cert *certificate.Certificate) control.VideoChannelOption {
	return func(channel *video.VideoChannel) error {
		channel.Owner = &video.VideoChannel_LocalShare_{
			LocalShare: &video.VideoChannel_LocalShare{
				Certificate: cert,
			},
		}
		return nil
	}
}

// WithRemoteShareOwner ...
func WithRemoteShareOwner(share *video.VideoChannel_RemoteShare) control.VideoChannelOption {
	return func(channel *video.VideoChannel) error {
		channel.Owner = &video.VideoChannel_RemoteShare_{
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
