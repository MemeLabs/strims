package main

import (
	"errors"
	"flag"
	"log"
	"net"
	"net/http"
	_ "net/http/pprof"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/nareix/joy5/format/rtmp"
	"go.uber.org/zap"
)

var addr = flag.String("addr", "0.0.0.0:8082", "bootstrap server listen address")

func main() {
	flag.Parse()
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
		vpn.WithInterface(vpn.NewWSInterface(logger, *addr)),
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

	test(logger, profileStore, networkController)

	select {}
}

func test(logger *zap.Logger, profileStore *dao.ProfileStore, ctl *service.NetworkController) {
	x := rtmpingress.NewTranscoder(logger)
	rtmp := rtmpingress.Server{
		Addr: "0.0.0.0:1935",
		HandleStream: func(a *rtmpingress.StreamAddr, c *rtmp.Conn, nc net.Conn) {
			logger.Debug("rtmp stream opened", zap.String("key", a.Key))

			v, err := service.NewVideoServer()
			if err != nil {
				logger.Debug("starting video server failed", zap.Error(err))
				if err := nc.Close(); err != nil {
					logger.Debug("closing rtmp net con failed", zap.Error(err))
				}
				return
			}

			go func() {
				if err := x.Transcode(a.URI, a.Key, "source", v); err != nil {
					logger.Debug("transcoder finished", zap.Error(err))
				}
			}()

			memberships, err := dao.GetNetworkMemberships(profileStore)
			if err != nil {
				logger.Debug("loading network memberships failed", zap.Error(err))

				v.Stop()

				if err := nc.Close(); err != nil {
					logger.Debug("closing rtmp net con failed", zap.Error(err))
				}
				return
			}

			for _, membership := range memberships {
				svc, ok := ctl.NetworkServices(dao.GetRootCert(membership.Certificate).Key)
				if !ok {
					logger.Debug("publishing video swarm failed", zap.Error(errors.New("unknown network")))
				}

				if err := v.PublishSwarm(svc); err != nil {
					logger.Debug("publishing video swarm failed", zap.Error(err))
				}
			}

			go func() {
				<-c.CloseNotify()
				logger.Debug("rtmp stream closed", zap.String("key", a.Key))
				v.Stop()
			}()
		},
	}
	go func() {
		if err := rtmp.Listen(); err != nil {
			logger.Fatal("rtmp server listen failed", zap.Error(err))
		}
	}()
}

func initProfileStore() (*dao.ProfileStore, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("failed to locate home directory: %s", err)
	}
	kv, err := bboltkv.NewStore(path.Join(homeDir, ".strims"))
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
