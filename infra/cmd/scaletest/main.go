package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
	"google.golang.org/protobuf/proto"
)

var (
	controllerIP string
	metricsPort  int
	debugPort    int
	wsPort       int
	webrtcPort   int
	rtmpPort     int
	invitesPort  int
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	flag.StringVar(&controllerIP, "controller-ip", "10.0.0.1", "IP of the node exposing svc")
	flag.IntVar(&metricsPort, "metrics-port", 30000, "svc metrics port")
	flag.IntVar(&debugPort, "debug-port", 30001, "svc debug port")
	flag.IntVar(&wsPort, "ws-port", 30002, "svc websocket port")
	flag.IntVar(&webrtcPort, "webrtc-port", 30003, "svc webrtc port")
	flag.IntVar(&rtmpPort, "rtmp-port", 1935, "svc RTMP port")
	flag.IntVar(&invitesPort, "invites-port", 30005, "svc invites port")
	flag.Parse()

	logger, err := zap.NewDevelopment()
	if err != nil {
		return err
	}

	host, err := newClient(logger)
	if err != nil {
		return err
	}

	ctx := context.TODO()

	log.Println("signing up...")
	signUpRequest := &authv1.SignUpRequest{Name: "host1", Password: "password"}
	signUpResponse := &authv1.SignUpResponse{}
	if err = host.Auth.SignUp(ctx, signUpRequest, signUpResponse); err != nil {
		return fmt.Errorf("unable to create host profile: %w", err)
	}

	log.Println("creating server...")
	createServerRequest := &networkv1.CreateServerRequest{Name: "test"}
	createServerResponse := &networkv1.CreateServerResponse{}
	if err = host.Network.CreateServer(ctx, createServerRequest, createServerResponse); err != nil {
		return fmt.Errorf("failed to create server: %w", err)
	}

	networkID := createServerResponse.GetNetwork().GetId()
	listPeersResponse := &networkv1.ListPeersResponse{}
	if err = host.Network.ListPeers(ctx, &networkv1.ListPeersRequest{NetworkId: networkID}, listPeersResponse); err != nil {
		return fmt.Errorf("error listing peers: %w", err)
	}

	log.Printf("granting peer invitation for: %d", networkID)
	grantPeerInvitationReq := &networkv1.GrantPeerInvitationRequest{
		Id:    listPeersResponse.Peers[0].Id,
		Count: 1000000,
	}
	if err = host.Network.GrantPeerInvitation(ctx, grantPeerInvitationReq, &networkv1.GrantPeerInvitationResponse{}); err != nil {
		return fmt.Errorf("unable to grant peer invitation: %w", err)
	}

	log.Println("creating invitation request...")
	createInvitationRequest := &networkv1.CreateInvitationRequest{NetworkId: networkID}
	createInvitationResponse := &networkv1.CreateInvitationResponse{}
	if err = host.Network.CreateInvitation(ctx, createInvitationRequest, createInvitationResponse); err != nil {
		return err
	}

	profile := signUpResponse.GetProfile()
	invitation := createInvitationResponse.GetInvitation()

	eg := errgroup.Group{}
	eg.Go(func() error {
		client, err := newClient(logger)
		if err != nil {
			return err
		}

		log.Println("signing up as majora")
		request := &authv1.SignUpRequest{Name: "majora", Password: "password"}
		if err = client.Auth.SignUp(ctx, request, &authv1.SignUpResponse{}); err != nil {
			return err
		}

		log.Println("creating network from invitation")
		createNetworkFromInvitationReq := &networkv1.CreateNetworkFromInvitationRequest{
			Invitation: &networkv1.CreateNetworkFromInvitationRequest_InvitationBytes{
				InvitationBytes: errutil.Must(proto.Marshal(invitation)),
			},
		}
		if err = client.Network.CreateNetworkFromInvitation(ctx, createNetworkFromInvitationReq, &networkv1.CreateNetworkFromInvitationResponse{}); err != nil {
			return err
		}

		log.Println("creating bootstrap client")
		bootstrapClient := &networkv1bootstrap.CreateBootstrapClientRequest{
			ClientOptions: &networkv1bootstrap.CreateBootstrapClientRequest_WebsocketOptions{
				WebsocketOptions: &networkv1bootstrap.BootstrapClientWebSocketOptions{
					Url: fmt.Sprintf("ws://%s:%d/%x", controllerIP, wsPort, profile.GetKey().GetPublic()),
				},
			},
		}
		if err = client.Bootstrap.CreateClient(ctx, bootstrapClient, &networkv1bootstrap.CreateBootstrapClientResponse{}); err != nil {
			return err
		}

		debugConfigReq := &debugv1.SetConfigRequest{
			Config: &debugv1.Config{
				EnableMockStreams:    true,
				MockStreamNetworkKey: dao.NetworkKey(createServerResponse.Network),
			},
		}
		if err = client.Debug.SetConfig(ctx, debugConfigReq, &debugv1.SetConfigResponse{}); err != nil {
			return err
		}

		return nil
	})

	if err = eg.Wait(); err != nil {
		return err
	}

	time.Sleep(time.Second)

	log.Println("mock stream starting")

	startMockStreamReq := &debugv1.StartMockStreamRequest{
		BitrateKbps:       6000,
		SegmentIntervalMs: 1000,
		TimeoutMs:         5 * 60 * 1000,
		NetworkKey:        dao.NetworkKey(createServerResponse.Network),
	}
	if err := host.Debug.StartMockStream(ctx, startMockStreamReq, &debugv1.StartMockStreamResponse{}); err != nil {
		return err
	}

	<-time.After(5 * time.Minute)

	return nil
}

func newClient(logger *zap.Logger) (*apis.FrontendClient, error) {
	c, resp, err := websocket.DefaultDialer.Dial(fmt.Sprintf("ws://%s:%d/api", controllerIP, wsPort), nil)
	if err != nil {
		if resp != nil {
			return nil, fmt.Errorf("failed to dial with %d status code: %w", resp.StatusCode, err)
		} else {
			return nil, err
		}
	}

	rpcClient, err := rpc.NewClient(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: httputil.NewDefaultWSReadWriter(c),
	})
	if err != nil {
		return nil, fmt.Errorf("error creating rpc client: %w", err)
	}

	return apis.NewFrontendClient(rpcClient), nil
}
