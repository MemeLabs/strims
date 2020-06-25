package main

import (
	"errors"
	"log"
	"net"
	"net/http"
	_ "net/http/pprof"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/nareix/joy5/format/rtmp"
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

	test(profileStore, networkController)

	select {}
}

func test(profileStore *dao.ProfileStore, ctl *service.NetworkController) {
	x := rtmpingress.Transcoder{}
	rtmp := rtmpingress.Server{
		Addr: "0.0.0.0:1935",
		HandleStream: func(a *rtmpingress.StreamAddr, c *rtmp.Conn, nc net.Conn) {
			log.Println("handling stream...")

			v, err := service.NewVideoServer()
			if err != nil {
				panic(err)
			}

			go x.Transcode(a.URI, a.Key, "source", v)

			memberships, err := dao.GetNetworkMemberships(profileStore)
			if err != nil {
				panic(err)
			}

			for _, membership := range memberships {
				svc, ok := ctl.NetworkServices(dao.GetRootCert(membership.Certificate).Key)
				if !ok {
					panic(errors.New("unknown network"))
				}

				if err := v.PublishSwarm(svc); err != nil {
					panic(err)
				}
			}
		},
	}
	go rtmp.Listen()
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
