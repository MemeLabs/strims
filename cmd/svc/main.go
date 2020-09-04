package main

import (
	"errors"
	"flag"
	"log"
	"net/http"
	_ "net/http/pprof"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rtmpingress"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.uber.org/zap"
)

var (
	profileDir    string
	addr          string
	metricsAddr   string
	rtmpAddr      string
	debugAddr     string
	webRTCPortMin uint
	webRTCPortMax uint
)

func init() {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("failed to locate home directory: %s", err)
	}

	flag.StringVar(&profileDir, "profile-dir", homeDir, "profile db location")
	flag.StringVar(&addr, "addr", ":8082", "bootstrap server listen address")
	flag.StringVar(&metricsAddr, "metrics-addr", ":1971", "metrics server listen address")
	flag.StringVar(&rtmpAddr, "rtmp-addr", ":1935", "rtmp server listen address")
	flag.StringVar(&debugAddr, "debug-addr", ":6060", "debug server listen address")
	flag.UintVar(&webRTCPortMin, "webrtc-port-min", 0, "webrtc ephemeral port range min")
	flag.UintVar(&webRTCPortMax, "webrtc-port-max", 0, "webrtc ephemeral port range max")
}

func main() {
	flag.Parse()
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	if debugAddr != "" {
		go func() {
			log.Println(http.ListenAndServe(debugAddr, nil))
		}()
	}

	if metricsAddr != "" {
		go func() {
			http.Handle("/metrics", promhttp.Handler())
			http.ListenAndServe(metricsAddr, nil)
		}()
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	// t, err := newManager(logger)
	// if err != nil {
	// 	panic(err)
	// }
	// t.Run()
	// return

	profileStore, err := initProfileStore()
	if err != nil {
		panic(err)
	}

	profile, err := dao.GetProfile(profileStore)
	if err != nil {
		panic(err)
	}

	host, err := vpn.NewHost(
		logger,
		profile.Key,
		vpn.WithNetworkBroker(vpn.NewNetworkBroker(logger)),
		vpn.WithInterface(vpn.NewWSInterface(logger, addr)),
		vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(
			logger,
			&vpn.WebRTCDialerOptions{
				PortMin: uint16(webRTCPortMin),
				PortMax: uint16(webRTCPortMax),
			},
		))),
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

	if rtmpAddr != "" {
		runRTMPServer(logger, profileStore, networkController)
	}

	select {}
}

func runRTMPServer(logger *zap.Logger, profileStore *dao.ProfileStore, ctl *service.NetworkController) {
	x := rtmpingress.NewTranscoder(logger)
	rtmp := rtmpingress.Server{
		Addr: rtmpAddr,
		HandleStream: func(a *rtmpingress.StreamAddr, c *rtmpingress.Conn) {
			logger.Debug("rtmp stream opened", zap.String("key", a.Key))

			v, err := service.NewVideoServer(logger)
			if err != nil {
				logger.Debug("starting video server failed", zap.Error(err))
				if err := c.Close(); err != nil {
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

				if err := c.Close(); err != nil {
					logger.Debug("closing rtmp net con failed", zap.Error(err))
				}
				return
			}

			for _, membership := range memberships {
				membership := membership
				go func() {
					svc, ok := ctl.NetworkServices(dao.GetRootCert(membership.Certificate).Key)
					if !ok {
						logger.Debug("publishing video swarm failed", zap.Error(errors.New("unknown network")))
					}

					if err := v.PublishSwarm(svc); err != nil {
						logger.Debug("publishing video swarm failed", zap.Error(err))
					}
				}()
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
	kv, err := bboltkv.NewStore(path.Join(profileDir, ".strims"))
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
