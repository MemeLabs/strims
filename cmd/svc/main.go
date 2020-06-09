package main

import (
	"log"
	"net/http"
	_ "net/http/pprof"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	go func() {
		log.Println(http.ListenAndServe("localhost:6060", nil))
	}()

	profileStore, err := initProfileStore()
	if err != nil {
		panic(err)
	}

	profile, err := profileStore.GetProfile()
	if err != nil {
		panic(err)
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	networkController := service.NewNetworksController(logger, profileStore)

	bootstrapService := service.NewBootstrapService(
		logger,
		profileStore,
		networkController,
		service.BootstrapServiceOptions{
			EnablePublishing: true,
		},
	)

	_, err = vpn.NewHost(
		logger,
		profile.Key,
		vpn.WithNetworkBroker(vpn.NewNetworkBroker()),
		vpn.WithInterface(vpn.NewWSInterface(logger, "0.0.0.0:8082")),
		vpn.WithInterface(vpn.NewWebRTCInterface(&vpn.WebRTCDialer{})),
		service.WithNetworkController(networkController),
		service.WithBootstrapService(bootstrapService),
	)
	if err != nil {
		panic(err)
	}

	select {}

	return
}

func initProfileStore() (*dao.ProfileStore, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("failed to locate home directory: %s", err)
	}
	kv, err := kv.NewKVStore(path.Join(homeDir, ".strims"))
	if err != nil {
		log.Fatalf("failed to open db: %s", err)
	}
	ds, err := dao.NewMetadataStore(kv)
	if err != nil {
		panic(err)
	}

	profiles, err := ds.GetProfiles()
	if err != nil {
		return nil, err
	}

	name := "test"
	pw := "test"

	if len(profiles) == 0 {
		_, profileStore, err := ds.CreateProfile(name, pw)
		return profileStore, err
	}

	_, profileStore, err := ds.LoadProfile(profiles[0].Id, pw)
	return profileStore, err
}
