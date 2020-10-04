package chat

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var chatSalt = []byte("chat")

// NewChatServer ...
func NewChatServer(logger *zap.Logger, svc *NetworkServices, key *pb.Key) (*ChatServer, error) {
	ps, err := NewPubSubServer(svc, key, chatSalt)
	if err != nil {
		return nil, err
	}

	s := &ChatServer{
		logger: logger,
		ps:     ps,
		events: make(chan *pb.ChatServerEvent),
	}

	go s.transformChatMessages(ps)

	return s, nil
}

// ChatServer ...
type ChatServer struct {
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *PubSubServer
	events    chan *pb.ChatServerEvent
}

// Close ...
func (s *ChatServer) Close() {
	s.closeOnce.Do(func() {
		s.ps.Close()
		close(s.events)
	})
}

// Events ...
func (s *ChatServer) Events() <-chan *pb.ChatServerEvent {
	return s.events
}

func (s *ChatServer) transformChatMessages(ps *PubSubServer) {
	for message := range ps.Messages() {
		// TODO: map source host id to nick... need to retain vpn.Message meatadata

		var e pb.ChatClientEvent
		if err := proto.Unmarshal(message.Body, &e); err != nil {
			s.logger.Error("failed to unmarshal message", zap.Error(err))
			continue
		}

		switch b := e.Body.(type) {
		case *pb.ChatClientEvent_Message_:
			s.logger.Debug("chat message received", zap.String("message", b.Message.Body))
			b.Message.ServerTime = time.Now().UnixNano() / int64(time.Millisecond)
			b.Message.Entities = entities.Extract(b.Message.Body)
		case *pb.ChatClientEvent_Open_:
			// TODO: add joining nicks
			// entities.AddNick(b.Open.ClientId)
		case *pb.ChatClientEvent_Close_:
			// TODO: remove leaving nicks
			// entities.RemoveNick(b.Close.ClientId)
		}

		b, err := proto.Marshal(&e)
		if err != nil {
			continue
		}

		if err := ps.Send(context.TODO(), "", b); err != nil {
			s.logger.Error("failed to write to swarm", zap.Error(err))
		}
	}

	s.Close()
}

// NewChatClient ...
func NewChatClient(logger *zap.Logger, svc *NetworkServices, key []byte) (*ChatClient, error) {
	ps, err := NewPubSubClient(svc, key, chatSalt)
	if err != nil {
		return nil, err
	}

	c := &ChatClient{
		logger: logger,
		ps:     ps,
		events: make(chan *pb.ChatClientEvent),
	}

	go c.readChatEvents(ps)

	return c, nil
}

// ChatClient ...
type ChatClient struct {
	logger    *zap.Logger
	closeOnce sync.Once
	ps        *PubSubClient
	events    chan *pb.ChatClientEvent
}

// Close ...
func (c *ChatClient) Close() {
	c.closeOnce.Do(func() {
		c.ps.Close()
		close(c.events)
	})
}

// Send ...
func (c *ChatClient) Send(msg *pb.ChatClientEvent_Message) error {
	b, err := proto.Marshal(&pb.ChatClientEvent{
		Body: &pb.ChatClientEvent_Message_{
			Message: &pb.ChatClientEvent_Message{
				SentTime: time.Now().UnixNano() / int64(time.Millisecond),
				Body:     msg.Body,
			},
		},
	})
	if err != nil {
		return err
	}

	return c.ps.Send(context.TODO(), "", b)
}

// Events ...
func (c *ChatClient) Events() <-chan *pb.ChatClientEvent {
	return c.events
}

func (c *ChatClient) readChatEvents(ps *PubSubClient) {
	for m := range ps.Messages() {
		e := &pb.ChatClientEvent{}
		if err := proto.Unmarshal(m.Body, e); err != nil {
			continue
		}
		c.events <- e
	}

	c.Close()
}
