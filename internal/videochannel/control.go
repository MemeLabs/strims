package videochannel

import (
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// Option ...
type Option func(channel *videov1.VideoChannel) error

type Control interface {
	GetChannel(id uint64) (*videov1.VideoChannel, error)
	ListChannels() ([]*videov1.VideoChannel, error)
	CreateChannel(opts ...Option) (*videov1.VideoChannel, error)
	UpdateChannel(id uint64, opts ...Option) (*videov1.VideoChannel, error)
	DeleteChannel(id uint64) error
}

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, store *dao.ProfileStore, observers *event.Observers) Control {
	return &control{
		logger:    logger,
		vpn:       vpn,
		store:     store,
		observers: observers,
	}
}

// Control ...
type control struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	store     *dao.ProfileStore
	observers *event.Observers
}

// GetChannel ...
func (c *control) GetChannel(id uint64) (*videov1.VideoChannel, error) {
	return dao.VideoChannels.Get(c.store, id)
}

// ListChannels ...
func (c *control) ListChannels() ([]*videov1.VideoChannel, error) {
	// TODO: enrich channel data with liveness?
	return dao.VideoChannels.GetAll(c.store)
}

// CreateChannel ...
func (c *control) CreateChannel(opts ...Option) (*videov1.VideoChannel, error) {
	channel, err := dao.NewVideoChannel(c.store)
	if err != nil {
		return nil, err
	}

	if err := channelOptionSlice(opts).Apply(channel); err != nil {
		return nil, err
	}

	if err := dao.VideoChannels.Insert(c.store, channel); err != nil {
		return nil, err
	}

	return channel, err
}

// UpdateChannel ...
func (c *control) UpdateChannel(id uint64, opts ...Option) (*videov1.VideoChannel, error) {
	var channel *videov1.VideoChannel
	err := c.store.Update(func(tx kv.RWTx) (err error) {
		channel, err = dao.VideoChannels.Get(tx, id)
		if err != nil {
			return err
		}

		if err := channelOptionSlice(opts).Apply(channel); err != nil {
			return err
		}

		return dao.VideoChannels.Update(c.store, channel)
	})
	if err != nil {
		return nil, err
	}

	c.observers.EmitGlobal(event.VideoChannelUpdate{Channel: channel})

	return channel, err
}

type channelOptionSlice []Option

func (s channelOptionSlice) Apply(channel *videov1.VideoChannel) error {
	for _, o := range s {
		if err := o(channel); err != nil {
			return err
		}
	}
	return nil
}

// WithDirectorySnippet ...
func WithDirectorySnippet(snippet *networkv1directory.ListingSnippet) Option {
	return func(channel *videov1.VideoChannel) error {
		channel.DirectoryListingSnippet = snippet
		return nil
	}
}

// WithLocalOwner ...
func WithLocalOwner(profileKey, networkKey []byte) Option {
	return func(channel *videov1.VideoChannel) error {
		channel.Owner = &videov1.VideoChannel_Local_{
			Local: &videov1.VideoChannel_Local{
				AuthKey:    profileKey,
				NetworkKey: networkKey,
			},
		}
		return nil
	}
}

// WithLocalShareOwner ...
func WithLocalShareOwner(cert *certificate.Certificate) Option {
	return func(channel *videov1.VideoChannel) error {
		channel.Owner = &videov1.VideoChannel_LocalShare_{
			LocalShare: &videov1.VideoChannel_LocalShare{
				Certificate: cert,
			},
		}
		return nil
	}
}

// WithRemoteShareOwner ...
func WithRemoteShareOwner(share *videov1.VideoChannel_RemoteShare) Option {
	return func(channel *videov1.VideoChannel) error {
		channel.Owner = &videov1.VideoChannel_RemoteShare_{
			RemoteShare: share,
		}
		return nil
	}
}

// DeleteChannel ...
func (c *control) DeleteChannel(id uint64) error {
	if err := dao.VideoChannels.Delete(c.store, id); err != nil {
		return err
	}

	c.observers.EmitGlobal(event.VideoChannelRemove{ID: id})

	return nil
}
