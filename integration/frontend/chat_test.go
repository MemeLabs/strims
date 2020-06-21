// +build !web

package frontend

import (
	"context"
	"fmt"
	"log"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"golang.org/x/sync/errgroup"
)

func TestChat(t *testing.T) {
	d, err := driver.NewNative()
	if err != nil {
		log.Fatal(err)
	}

	type state struct {
		client  *rpc.Client
		profile pb.CreateProfileResponse
		vpn     pb.StartVPNResponse
	}

	a := &state{client: d.Client(&driver.ClientOptions{
		VPNServerAddr: "0.0.0.0:8083",
	})}
	b := &state{client: td.Client(&driver.ClientOptions{})}
	c := &state{client: td.Client(&driver.ClientOptions{})}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	profile := &pb.CreateProfileRequest{
		Name:     "testa",
		Password: "password",
	}
	if err := a.client.CallUnary(ctx, "createProfile", profile, &a.profile); err != nil {
		log.Fatal(err)
	}

	vpn := &pb.StartVPNRequest{
		EnableBootstrapPublishing: true,
	}
	if err := a.client.CallUnary(ctx, "startVPN", vpn, &a.vpn); err != nil {
		log.Fatal(err)
	}

	time.Sleep(time.Second)

	initClient := func(s *state, name string) error {
		profile = &pb.CreateProfileRequest{
			Name:     name,
			Password: "password",
		}
		if err := s.client.CallUnary(ctx, "createProfile", profile, &s.profile); err != nil {
			return err
		}

		bootstrapClient := &pb.CreateBootstrapClientRequest{
			ClientOptions: &pb.CreateBootstrapClientRequest_WebsocketOptions{
				WebsocketOptions: &pb.BootstrapClientWebSocketOptions{
					Url: "ws://localhost:8083/test-bootstrap",
				},
			},
		}
		if err := s.client.CallUnary(ctx, "createBootstrapClient", bootstrapClient, &pb.CreateBootstrapClientResponse{}); err != nil {
			return err
		}

		if err := s.client.CallUnary(ctx, "startVPN", &pb.StartVPNRequest{}, &s.vpn); err != nil {
			return err
		}

		return nil
	}

	var g errgroup.Group
	g.Go(func() error { return initClient(b, "testb") })
	g.Go(func() error { return initClient(c, "testc") })
	if err := g.Wait(); err != nil {
		log.Fatal(err)
	}

	time.Sleep(time.Second)

	bootstrapPeersRes := &pb.GetBootstrapPeersResponse{}
	if err := b.client.CallUnary(ctx, "getBootstrapPeers", &pb.GetBootstrapPeersRequest{}, bootstrapPeersRes); err != nil {
		log.Fatal(err)
	}

	if len(bootstrapPeersRes.Peers) == 0 {
		log.Fatal("received 0 bootstrap peers")
	}

	createNetworkReq := &pb.CreateNetworkRequest{
		Name: "test",
	}
	createNetworkRes := &pb.CreateNetworkResponse{}
	if err := b.client.CallUnary(ctx, "createNetwork", createNetworkReq, createNetworkRes); err != nil {
		log.Fatal(err)
	}

	publishReq := &pb.PublishNetworkToBootstrapPeerRequest{
		Key:     bootstrapPeersRes.Peers[0].Key,
		Network: createNetworkRes.Network,
	}
	if err := b.client.CallUnary(ctx, "publishNetworkToBootstrapPeer", publishReq, &pb.PublishNetworkToBootstrapPeerResponse{}); err != nil {
		log.Fatal(err)
	}

	invitationReq := &pb.CreateNetworkInvitationRequest{
		SigningKey:  createNetworkRes.Network.Key,
		SigningCert: createNetworkRes.Network.Certificate,
		NetworkName: createNetworkRes.Network.Name,
	}
	invitationRes := &pb.CreateNetworkInvitationResponse{}
	if err := b.client.CallUnary(ctx, "createNetworkInvitation", invitationReq, invitationRes); err != nil {
		log.Fatal(err)
	}

	createInvitationReq := &pb.CreateNetworkMembershipFromInvitationRequest{
		Invitation: &pb.CreateNetworkMembershipFromInvitationRequest_InvitationBytes{
			InvitationBytes: invitationRes.InvitationBytes,
		},
	}
	if err := c.client.CallUnary(ctx, "createNetworkMembershipFromInvitation", createInvitationReq, &pb.CreateNetworkMembershipFromInvitationResponse{}); err != nil {
		log.Fatal(err)
	}

	createChatServerReq := &pb.CreateChatServerRequest{
		NetworkKey: createNetworkRes.Network.Key.Public,
		ChatRoom: &pb.ChatRoom{
			Name: "test",
		},
	}
	createChatServerRes := &pb.CreateChatServerResponse{}
	if err := c.client.CallUnary(ctx, "createChatServer", createChatServerReq, createChatServerRes); err != nil {
		log.Fatal(err)
	}

	openChatServerReq := &pb.OpenChatServerRequest{
		Server: createChatServerRes.ChatServer,
	}
	chatServerEvents := make(chan *pb.ChatServerEvent, 1)
	if err := c.client.CallStreaming(ctx, "openChatServer", openChatServerReq, chatServerEvents); err != nil {
		log.Fatal(err)
	}

	go func() {
		for e := range chatServerEvents {
			log.Println("chat server event", e)
		}
	}()

	time.Sleep(time.Second)

	openChatClientReq := &pb.OpenChatClientRequest{
		NetworkKey: createNetworkRes.Network.Key.Public,
		ServerKey:  createChatServerRes.ChatServer.Key.Public,
	}
	chatClientEvents := make(chan *pb.ChatClientEvent, 1)
	if err := b.client.CallStreaming(ctx, "openChatClient", openChatClientReq, chatClientEvents); err != nil {
		log.Fatal(err)
	}

	sendMessages := func(ctx context.Context, clientID uint64) {
		for {
			t := time.NewTicker(time.Second)

			select {
			case now := <-t.C:
				callChatClientReq := &pb.CallChatClientRequest{
					ClientId: clientID,
					Body: &pb.CallChatClientRequest_Message_{
						Message: &pb.CallChatClientRequest_Message{
							Time: now.UnixNano(),
							Body: fmt.Sprintf("test message %s", now.UTC().Format(time.RFC3339)),
						},
					},
				}
				if err := b.client.Call(ctx, "callChatClient", callChatClientReq); err != nil {
					log.Fatal(err)
				}
			case <-ctx.Done():
				return
			}
		}
	}

	done := make(chan struct{})

	go func() {
		ctx, cancel := context.WithCancel(context.Background())
		defer cancel()

		for e := range chatClientEvents {
			switch b := e.Body.(type) {
			case *pb.ChatClientEvent_Open_:
				go sendMessages(ctx, b.Open.ClientId)
			case *pb.ChatClientEvent_Message_:
				log.Println("chat client message", b.Message.Body)
				close(done)
			case *pb.ChatClientEvent_Close_:
				return
			}
		}
		log.Println("chat client closed")
	}()

	<-done

	d.Close()
}
