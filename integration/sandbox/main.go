package main

import (
	"context"
	"flag"
	"fmt"
	"os"
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

	conf := driver.Config{
		VpnAddr: *addr,
		Log:     os.Stdout,
	}

	drvr := driver.Setup(conf)
	sugar.Debug("driver established")

	profileRequest := &pb.CreateProfileRequest{
		Name:     "jbpratt",
		Password: "ilovemajora",
	}
	profileResponse := &pb.CreateProfileResponse{}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	if err := drvr.Client.CallUnary(ctx, "createProfile", profileRequest, profileResponse); err != nil {
		sugar.Fatal("failed to call createProfile", err)
	}
	sugar.Debugf("createProfile response: %v", profileResponse)

	networkRequest := &pb.CreateNetworkRequest{Name: "strims"}
	networkResponse := &pb.CreateNetworkResponse{}

	if err := drvr.Client.CallUnary(ctx, "createNetwork", networkRequest, networkResponse); err != nil {
		sugar.Fatal("failed to call createNetwork", err)
	}
	sugar.Debugf("createNetwork: %v", networkResponse)

	for i := 0; i < *peers; i++ {
		bootstrapRequest := &pb.CreateBootstrapClientRequest{
			ClientOptions: &pb.CreateBootstrapClientRequest_WebsocketOptions{
				WebsocketOptions: &pb.BootstrapClientWebSocketOptions{
					Url: fmt.Sprintf("ws://%s", conf.VpnAddr),
				},
			},
		}
		bootstrapResponse := &pb.CreateBootstrapClientResponse{}

		if err := drvr.Client.CallUnary(ctx, "createBootstrapClient", bootstrapRequest, bootstrapResponse); err != nil {
			sugar.Fatal("failed to call createBootstrapClient", err)
		}
		sugar.Debugf("createBootstrapClient (%d): %v", i, bootstrapResponse)
	}

	sugar.Infof("listening on %q", conf.VpnAddr)
	defer drvr.Teardown()
	select {}
}
