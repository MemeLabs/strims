package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

var (
	peers = flag.Int("peers", 1, "amount of peers to start")
	addr  = flag.String("addr", "0.0.0.0:9999", "address to setup from")
)

func main() {
	flag.Parse()
	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	defer logger.Sync()

	sugar := logger.Sugar()

	conf := driver.Config{VpnAddr: *addr}
	drvr := driver.Setup(conf)
	sugar.Debug("driver established")

	conf.File = drvr.File
	conf.Store = drvr.Store

	profileRequest := &pb.CreateProfileRequest{
		Name:     "jbpratt",
		Password: "ilovemajora",
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	// create profile
	profileResponse, err := drvr.Frontend.CreateProfile(ctx, profileRequest)
	if err != nil {
		log.Fatal(err)
	}
	sugar.Debugf("createProfile response: %v", profileResponse)

	// create network
	networkRequest := &pb.CreateNetworkRequest{Name: "strims"}
	networkResponse, err := drvr.Frontend.CreateNetwork(ctx, networkRequest)
	if err != nil {
		log.Fatal(err)
	}
	sugar.Debugf("createNetwork: %v", networkResponse)

	bootstrapRequest := &pb.CreateBootstrapClientRequest{
		ClientOptions: &pb.CreateBootstrapClientRequest_WebsocketOptions{
			WebsocketOptions: &pb.BootstrapClientWebSocketOptions{
				Url: fmt.Sprintf("ws://%s", conf.VpnAddr),
			},
		},
	}
	// add boostrap client
	bootstrapResponse, err := drvr.Frontend.CreateBootstrapClient(ctx, bootstrapRequest)
	if err != nil {
		log.Fatal(err)
	}
	sugar.Debugf("createBootstrapClient response: %v", bootstrapResponse)

	for i := 0; i < *peers; i++ {
		_ = driver.Setup(conf)
		// start host / bootstrap client ??
	}
	startVPNRequest := &pb.StartVPNRequest{}
	startVPNResponse, err := drvr.Frontend.StartVPN(ctx, startVPNRequest)
	if err != nil {
		log.Fatal(err)
	}

	sugar.Debugf("startVPN response: %v", startVPNResponse)
	sugar.Infof("listening on %q", conf.VpnAddr)
	defer drvr.Teardown()
	select {}
}
