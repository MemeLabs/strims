package autoseed

import (
	"bytes"
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	autoseedv1 "github.com/MemeLabs/go-ppspp/pkg/apis/autoseed/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"go.uber.org/zap"
)

type Control interface {
	Run()
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	transfer transfer.Control,
) Control {
	return &control{
		ctx:      ctx,
		logger:   logger,
		store:    store,
		transfer: transfer,

		events:  observers.Chan(),
		runners: map[uint64]*runner{},
	}
}

// Control ...
type control struct {
	ctx      context.Context
	logger   *zap.Logger
	store    *dao.ProfileStore
	transfer transfer.Control

	events  chan any
	lock    sync.Mutex
	config  *autoseedv1.Config
	runners map[uint64]*runner
}

// Run ...
func (c *control) Run() {
	go c.loadConfig()
	go c.loadRules()

	for {
		select {
		case e := <-c.events:
			switch e := e.(type) {
			case *autoseedv1.ConfigChangeEvent:
				c.handleConfigChange(e.Config)
			case *autoseedv1.RuleChangeEvent:
				c.handleRuleChange(e.Rule)
			case *autoseedv1.RuleDeleteEvent:
				c.handleRuleDelete(e.Rule)
			case event.DirectoryEvent:
				c.handleDirectoryEvent(e)
			}
		case <-c.ctx.Done():
			return
		}
	}
}

func (c *control) loadConfig() {
	config, err := dao.AutoseedConfig.Get(c.store)
	if err != nil {
		c.logger.Warn("failed to load autoseed config", zap.Error(err))
		return
	}

	c.lock.Lock()
	defer c.lock.Unlock()

	c.applyConfig(config)
}

func (c *control) loadRules() {
	rules, err := dao.AutoseedRules.GetAll(c.store)
	if err != nil {
		c.logger.Warn("failed to load autoseed rules", zap.Error(err))
		return
	}

	c.lock.Lock()
	defer c.lock.Unlock()

	for _, r := range rules {
		go c.applyRule(r)
	}
}

func (c *control) handleConfigChange(config *autoseedv1.Config) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.applyConfig(config)
}

func (c *control) handleRuleChange(rule *autoseedv1.Rule) {
	c.lock.Lock()
	defer c.lock.Unlock()

	c.applyRule(rule)
}

func (c *control) handleRuleDelete(rule *autoseedv1.Rule) {
	c.lock.Lock()
	defer c.lock.Unlock()

	r, ok := c.runners[rule.Id]
	if ok && r.running {
		c.stopSwarm(r)
		delete(c.runners, rule.Id)
	}
}

func (c *control) handleDirectoryEvent(event event.DirectoryEvent) {
	c.lock.Lock()
	defer c.lock.Unlock()

	for _, e := range event.Broadcast.Events {
		switch b := e.Body.(type) {
		case *networkv1directory.Event_ListingChange_:
			c.tryStartSwarm(event.NetworkKey, b.ListingChange.Id, b.ListingChange.Listing)
		case *networkv1directory.Event_Unpublish_:
			c.tryStopSwarm(b.Unpublish.Id)
		}
	}
}

func (c *control) applyConfig(config *autoseedv1.Config) {
	stopRunning := c.config.GetEnable() && !config.Enable

	c.config = config

	if stopRunning {
		for _, r := range c.runners {
			if r.running {
				c.stopSwarm(r)
			}
		}
	}
}

func (c *control) applyRule(rule *autoseedv1.Rule) {
	r, ok := c.runners[rule.Id]
	if !ok {
		r = &runner{}
		c.runners[rule.Id] = r
	} else {
		if !r.swarmID.Equals(rule.SwarmId) {
			c.stopSwarm(r)
		}
		if r.running && !bytes.Equal(r.networkKey, rule.NetworkKey) {
			c.transfer.Publish(r.transferID, rule.NetworkKey)
		}
	}

	r.id = rule.Id
	r.networkKey = rule.NetworkKey
	r.swarmID = rule.SwarmId
	r.salt = rule.Salt
}

func (c *control) tryStartSwarm(networkKey []byte, listingID uint64, l *networkv1directory.Listing) {
	if !c.config.Enable {
		return
	}

	m := l.GetMedia()
	if m == nil {
		return
	}
	uri, err := ppspp.ParseURI(m.GetSwarmUri())
	if err != nil {
		return
	}

	for _, r := range c.runners {
		if bytes.Equal(networkKey, r.networkKey) && uri.ID.Equals(r.swarmID) && !r.running {
			r.transferID = c.startSwarm(r, uri)
			r.listingID = listingID
			r.running = true
		}
	}
}

func (c *control) tryStopSwarm(listingID uint64) {
	for _, r := range c.runners {
		if r.listingID == listingID && r.running {
			c.stopSwarm(r)
			r.transferID = transfer.NilID
			r.listingID = 0
			r.running = false
		}
	}
}

func (c *control) startSwarm(r *runner, uri *ppspp.URI) transfer.ID {
	c.logger.Debug(
		"starting swarm",
		zap.Uint64("rule", r.id),
		zap.Stringer("swarm", r.swarmID),
		logutil.ByteHex("network", r.networkKey),
	)

	transferID, _, ok := c.transfer.Find(r.swarmID, r.salt)
	if !ok {
		opt := uri.Options.SwarmOptions()
		opt.LiveWindow = (32 * 1024 * 1024) / opt.ChunkSize

		swarm, err := ppspp.NewSwarm(uri.ID, opt)
		if err != nil {
			return transfer.NilID
		}

		transferID = c.transfer.Add(swarm, r.salt)
	}

	c.logger.Debug(
		"publishing transfer",
		zap.Uint64("rule", r.id),
		zap.Stringer("swarm", uri.ID),
		logutil.ByteHex("transfer", transferID[:]),
		logutil.ByteHex("network", r.networkKey),
	)
	c.transfer.Publish(transferID, r.networkKey)

	return transferID
}

func (c *control) stopSwarm(r *runner) {
	c.logger.Debug(
		"stopping swarm",
		zap.Uint64("rule", r.id),
		zap.Stringer("swarm", r.swarmID),
		logutil.ByteHex("network", r.networkKey),
	)

	c.transfer.Remove(r.transferID)
}

type runner struct {
	id         uint64
	networkKey []byte
	swarmID    ppspp.SwarmID
	salt       []byte

	running    bool
	listingID  uint64
	transferID transfer.ID
}
