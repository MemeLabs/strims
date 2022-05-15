// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vpn

import (
	"context"
	"sync"

	"github.com/MemeLabs/strims/pkg/kademlia"
)

// NewChannel ...
func NewChannel(ctx context.Context, network *Network, dstID kademlia.ID, dstPort uint16, srcPort uint16) *Channel {
	ctx, cancel := context.WithCancel(ctx)
	ch := &Channel{
		ctx:      ctx,
		close:    cancel,
		messages: make(chan *Message),
		network:  network,
		dstID:    dstID,
		dstPort:  dstPort,
		srcPort:  srcPort,
	}

	go ch.run()

	return ch
}

// Channel ...
type Channel struct {
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	messages  chan *Message
	network   *Network
	dstID     kademlia.ID
	dstPort   uint16
	srcPort   uint16
}

// Write ...
func (c *Channel) Write(b []byte) (int, error) {
	if err := c.network.Send(c.dstID, c.dstPort, c.srcPort, b); err != nil {
		return 0, err
	}
	return len(b), nil
}

// Read ...
func (c *Channel) Read(b []byte) (int, error) {
	select {
	case <-c.ctx.Done():
		return 0, c.ctx.Err()
	case m := <-c.messages:
		if len(m.Body) > len(b) {
			return 0, errBufferTooSmall
		}
		copy(b, m.Body)
		return len(m.Body), nil
	}
}

// Close ...
func (c *Channel) Close() error {
	c.closeOnce.Do(func() { c.close() })
	return nil
}

func (c *Channel) run() {
	if err := c.network.SetHandler(c.srcPort, channelMessageHandler{c}); err != nil {
		c.Close()
		return
	}

	<-c.ctx.Done()
	c.network.RemoveHandler(c.srcPort)
}

type channelMessageHandler struct {
	*Channel
}

func (c channelMessageHandler) HandleMessage(m *Message) error {
	c.messages <- m
	return nil
}
