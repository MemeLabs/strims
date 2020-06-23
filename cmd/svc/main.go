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

	profile, err := dao.GetProfile(profileStore)
	if err != nil {
		panic(err)
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	host, err := vpn.NewHost(
		logger,
		profile.Key,
		vpn.WithNetworkBroker(vpn.NewNetworkBroker(logger)),
		vpn.WithInterface(vpn.NewWSInterface(logger, "0.0.0.0:8082")),
		vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger))),
	)
	if err != nil {
		panic(err)
	}

	networkController, err := service.NewNetworkController(logger, host, profileStore)
	if err != nil {
		panic(err)
	}

	bootstrapService := service.NewBootstrapService(
		logger,
		host,
		profileStore,
		networkController,
		service.BootstrapServiceOptions{
			EnablePublishing: true,
		},
	)

	_ = bootstrapService

	select {}
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

	profiles, err := dao.GetProfileSummaries(ds)
	if err != nil {
		return nil, err
	}

	name := "test"
	pw := "test"

	if len(profiles) == 0 {
		_, profileStore, err := dao.CreateProfile(ds, name, pw)
		return profileStore, err
	}

	_, profileStore, err := dao.LoadProfile(ds, profiles[0].Id, pw)
	return profileStore, err
}
