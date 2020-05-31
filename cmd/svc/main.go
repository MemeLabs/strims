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

	profileID := 4303395559
	key := []byte{127, 190, 70, 244, 47, 123, 114, 24, 159, 15, 221, 25, 228, 167, 124, 142, 211, 181, 221, 93, 127, 68, 234, 112, 77, 144, 43, 75, 241, 229, 201, 51}

	_, profileStore, err := ds.LoadSession(uint64(profileID), dao.NewStorageKeyFromBytes(key))
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

	_, err = vpn.NewHost(
		logger,
		profile.Key,
		vpn.WithNetworkBroker(vpn.NewNetworkBroker()),
		vpn.WithInterface(vpn.NewWSInterface(logger, "0.0.0.0:8082")),
		vpn.WithInterface(vpn.NewWebRTCInterface(&vpn.WebRTCDialer{})),
		service.WithNetworkController(service.NewNetworksController(logger, profileStore)),
	)
	if err != nil {
		panic(err)
	}

	// h := &vpn.Host{
	// 	Interfaces: []vpn.Interface{
	// 		&wsInterface{
	// 			Address: "0.0.0.0:8082",
	// 		},
	// 	},
	// }

	// _ = h

	// m, err := newManager(logger)
	// if err != nil {
	// 	panic(err)
	// }
	// go m.Run()

	// var d webRTCDialer
	// client := vpn.NewClient(profileStore, h, d.DialWebRTC)
	// client.Start()

	select {}

	return
}
