package service

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
)

var chatSalt = []byte("chat")

// NewChatServer ...
func NewChatServer(ctx context.Context, svc *NetworkServices, key *pb.Key) (*ChatServer, error) {
	ctx, cancel := context.WithCancel(ctx)

	ps, err := NewPubSubServer(ctx, svc, key, chatSalt)
	if err != nil {
		cancel()
		return nil, err
	}

	events := make(chan *pb.ChatServerEvent)
	go func() {
		transformChatMessages(ctx, ps)
		cancel()
		close(events)
	}()

	return &ChatServer{
		close:  cancel,
		events: events,
	}, nil
}

// ChatServer ...
type ChatServer struct {
	close  context.CancelFunc
	events chan *pb.ChatServerEvent
}

// Close ...
func (s *ChatServer) Close() {
	s.close()
}

// Events ...
func (s *ChatServer) Events() <-chan *pb.ChatServerEvent {
	return s.events
}

func transformChatMessages(ctx context.Context, sp *PubSubServer) {
	for {
		select {
		case <-ctx.Done():
			return
		case p := <-sp.Publishes():
			// TODO: chat output schema
			// TODO: use strims chat parser?
			// TODO: map source host id to nick... need to retain vpn.Message meatadata
			sp.Send("", p.Body)
		}
	}
}

// NewChatClient ...
func NewChatClient(ctx context.Context, svc *NetworkServices, key []byte) (*ChatClient, error) {
	ctx, cancel := context.WithCancel(ctx)

	ps, err := NewPubSubClient(ctx, svc, key, chatSalt)
	if err != nil {
		cancel()
		return nil, err
	}

	events := make(chan *pb.ChatClientEvent)
	go func() {
		readChatEvents(ctx, ps, events)
		cancel()
		close(events)
	}()

	return &ChatClient{
		close:  cancel,
		ps:     ps,
		events: events,
	}, nil
}

// ChatClient ...
type ChatClient struct {
	close  context.CancelFunc
	ps     *PubSubClient
	events chan *pb.ChatClientEvent
}

// Close ...
func (c *ChatClient) Close() {
	c.close()
}

// Send ...
func (c *ChatClient) Send(msg *pb.ChatClientEvent_Message) error {
	b, err := proto.Marshal(&pb.ChatClientEvent{
		Body: &pb.ChatClientEvent_Message_{
			Message: msg,
		},
	})
	if err != nil {
		return err
	}

	return c.ps.Publish("", b)
}

// Events ...
func (c *ChatClient) Events() <-chan *pb.ChatClientEvent {
	return c.events
}

func readChatEvents(ctx context.Context, ps *PubSubClient, events chan *pb.ChatClientEvent) {
	for {
		select {
		case <-ctx.Done():
			return
		case psm, ok := <-ps.Messages():
			if !ok {
				return
			}

			e := &pb.ChatClientEvent{}
			if err := proto.Unmarshal(psm.Body, e); err != nil {
				continue
			}
			events <- e
		}
	}
}
