// +build !web

package frontend

import (
	"context"
	"fmt"
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
		t.Error(err)
	}

	type state struct {
		client  *rpc.Client
		profile pb.CreateProfileResponse
	}

	a := &state{client: d.Client(&driver.ClientOptions{
		VPNServerAddr: "0.0.0.0:8084",
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
		t.Error(err)
	}

	vpn := &pb.StartVPNRequest{
		EnableBootstrapPublishing: true,
	}
	if err := a.client.CallStreaming(ctx, "startVPN", vpn, make(chan *pb.NetworkEvent)); err != nil {
		t.Error(err)
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
					Url: "ws://localhost:8084/test-bootstrap",
				},
			},
		}
		if err := s.client.CallUnary(ctx, "createBootstrapClient", bootstrapClient, &pb.CreateBootstrapClientResponse{}); err != nil {
			return err
		}

		if err := s.client.CallStreaming(ctx, "startVPN", &pb.StartVPNRequest{}, make(chan *pb.NetworkEvent)); err != nil {
			return err
		}

		return nil
	}

	var g errgroup.Group
	g.Go(func() error { return initClient(b, "testb") })
	g.Go(func() error { return initClient(c, "testc") })
	if err := g.Wait(); err != nil {
		t.Error(err)
	}

	time.Sleep(time.Second)

	bootstrapPeersRes := &pb.GetBootstrapPeersResponse{}
	if err := b.client.CallUnary(ctx, "getBootstrapPeers", &pb.GetBootstrapPeersRequest{}, bootstrapPeersRes); err != nil {
		t.Error(err)
	}

	if len(bootstrapPeersRes.Peers) == 0 {
		t.Error("received 0 bootstrap peers")
	}

	createNetworkReq := &pb.CreateNetworkRequest{
		Name: "test",
	}
	createNetworkRes := &pb.CreateNetworkResponse{}
	if err := b.client.CallUnary(ctx, "createNetwork", createNetworkReq, createNetworkRes); err != nil {
		t.Error(err)
	}

	publishReq := &pb.PublishNetworkToBootstrapPeerRequest{
		HostId:  bootstrapPeersRes.Peers[0].HostId,
		Network: createNetworkRes.Network,
	}
	if err := b.client.CallUnary(ctx, "publishNetworkToBootstrapPeer", publishReq, &pb.PublishNetworkToBootstrapPeerResponse{}); err != nil {
		t.Error(err)
	}

	invitationReq := &pb.CreateNetworkInvitationRequest{
		SigningKey:  createNetworkRes.Network.Key,
		SigningCert: createNetworkRes.Network.Certificate,
		NetworkName: createNetworkRes.Network.Name,
	}
	invitationRes := &pb.CreateNetworkInvitationResponse{}
	if err := b.client.CallUnary(ctx, "createNetworkInvitation", invitationReq, invitationRes); err != nil {
		t.Error(err)
	}

	createInvitationReq := &pb.CreateNetworkMembershipFromInvitationRequest{
		Invitation: &pb.CreateNetworkMembershipFromInvitationRequest_InvitationBytes{
			InvitationBytes: invitationRes.InvitationBytes,
		},
	}
	if err := c.client.CallUnary(ctx, "createNetworkMembershipFromInvitation", createInvitationReq, &pb.CreateNetworkMembershipFromInvitationResponse{}); err != nil {
		t.Error(err)
	}

	createChatServerReq := &pb.CreateChatServerRequest{
		NetworkKey: createNetworkRes.Network.Key.Public,
		ChatRoom: &pb.ChatRoom{
			Name: "test",
		},
	}
	createChatServerRes := &pb.CreateChatServerResponse{}
	if err := c.client.CallUnary(ctx, "createChatServer", createChatServerReq, createChatServerRes); err != nil {
		t.Error(err)
	}

	openChatServerReq := &pb.OpenChatServerRequest{
		Server: createChatServerRes.ChatServer,
	}
	chatServerEvents := make(chan *pb.ChatServerEvent, 1)
	if err := c.client.CallStreaming(ctx, "openChatServer", openChatServerReq, chatServerEvents); err != nil {
		t.Error(err)
	}

	go func() {
		for e := range chatServerEvents {
			t.Log("chat server event", e)
		}
	}()

	time.Sleep(time.Second)

	openChatClientReq := &pb.OpenChatClientRequest{
		NetworkKey: createNetworkRes.Network.Key.Public,
		ServerKey:  createChatServerRes.ChatServer.Key.Public,
	}
	chatClientEvents := make(chan *pb.ChatClientEvent, 1)
	if err := b.client.CallStreaming(ctx, "openChatClient", openChatClientReq, chatClientEvents); err != nil {
		t.Error(err)
	}

	sendMessages := func(ctx context.Context, clientID uint64) {
		for {
			ticker := time.NewTicker(time.Second)

			select {
			case now := <-ticker.C:
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
					t.Error(err)
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
				t.Log("chat client message", b.Message.Body)
				close(done)
			case *pb.ChatClientEvent_Close_:
				return
			}
		}
		t.Log("chat client closed")
	}()

	<-done

	d.Close()
}
