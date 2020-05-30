// +build js

package main

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"log"
	mrand "math/rand"
	"runtime"
	"strconv"
	"strings"
	"syscall/js"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/gobridge"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

func init() {
	go runGC()
	seedMathRand()
}

func runGC() {
	for range time.NewTicker(10 * time.Second).C {
		runtime.GC()
	}
}

func seedMathRand() {
	var t [8]byte
	if _, err := rand.Read(t[:]); err != nil {
		panic(err)
	}
	mrand.Seed(int64(binary.LittleEndian.Uint64(t[:])))
}

func consoleLog(args ...interface{}) {
	js.Global().Get("console").Call("log", args...)
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	gobridge.RegisterCallback("init", func(this js.Value, args []js.Value) (interface{}, error) {
		service := args[0].String()
		bridge := args[1]

		init, ok := map[string]func(js.Value, *wasmio.Bus){
			"default": initDefault,
			"broker":  initBroker,
		}[service]
		if !ok {
			return nil, errors.New("unknown service")
		}

		go init(bridge, wasmio.NewBus(bridge, service))

		return nil, nil
	})

	select {}
}

func newLogger(bridge js.Value) *zap.Logger {
	encoderCfg := zapcore.EncoderConfig{
		MessageKey:     "msg",
		LevelKey:       "level",
		NameKey:        "logger",
		CallerKey:      "caller",
		EncodeLevel:    zapcore.LowercaseLevelEncoder,
		EncodeTime:     zapcore.ISO8601TimeEncoder,
		EncodeDuration: zapcore.StringDurationEncoder,
		EncodeCaller:   zapcore.ShortCallerEncoder,
	}
	sink := wasmio.NewZapSink(bridge)
	core := zapcore.NewCore(zapcore.NewJSONEncoder(encoderCfg), sink, zap.DebugLevel)
	return zap.New(core, zap.WithCaller(true))
}

func initDefault(bridge js.Value, bus *wasmio.Bus) {
	logger := newLogger(bridge)
	svc, err := service.New(service.Options{
		Store:  wasmio.NewKVStore(bridge),
		Logger: logger,
		VPNOptions: []vpn.HostOption{
			vpn.WithNetworkBroker(vpn.NewBrokerClient(wasmio.NewWorkerProxy(bridge, "broker"))),
			vpn.WithInterface(vpn.NewWSInterface(logger, bridge)),
			vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger, bridge))),
		},
	})
	if err != nil {
		log.Fatalf("error creating service: %s", err)
	}
	rpc.NewHost(svc).Handle(context.Background(), bus, bus)
}

func initBroker(bridge js.Value, bus *wasmio.Bus) {
	rpc.NewHost(vpn.NewBrokerService()).Handle(context.Background(), bus, bus)
}

func testRTC(bridge js.Value) {
	// conn := wasmio.NewWebRTCProxy(bridge, []string{"dht", "ppspp"})
	// offer, err := conn.CreateOffer()
	// if err != nil {
	// 	panic(err)
	// }
	// conn.SetLocalDescription(offer)
	// log.Println(offer, err)

	// c, _ := conn.ICECandidates()
	// test, _ := json.Marshal(c)
	// log.Println(string(test))
}

func testKV(bridge js.Value) {
	store := wasmio.NewKVStore(bridge)

	err := store.CreateStoreIfNotExists("test")
	if err != nil {
		panic(err)
	}

	log.Println("updating test to foo")
	store.Update("test", func(tx dao.Tx) error {
		return tx.Put("test", []byte("foo"))
	})

	log.Println("checking test")
	store.View("test", func(tx dao.Tx) error {
		v, err := tx.Get("test")
		if err != nil {
			return err
		}
		log.Println(string(v))
		return nil
	})

	log.Println("read/write/read transaction")
	store.Update("test", func(tx dao.Tx) error {
		v, err := tx.Get("test")
		if err != nil {
			return err
		}
		log.Println(string(v))

		if err := tx.Put("test", []byte("bar")); err != nil {
			return err
		}

		v, err = tx.Get("test")
		if err != nil {
			return err
		}
		log.Println(string(v))

		return nil
	})

	// v := make([]byte, 128)

	// var rseed [16]byte
	// rand.Read(rseed[:])
	// rng, _ := mpc.NewAESRNG(rseed[:])
	// rng.Read(v)

	// err := kv.Put("foo", v)
	// if err != nil {
	// 	panic(err)
	// }

	// b2, err := kv.Get("foo")
	// if err != nil {
	// 	panic(err)
	// }

	// log.Println(v)
	// log.Println(b2)

	// if bytes.Compare(v, b2) != 0 {
	// 	panic("the bytes don't match...")
	// }

	// err = kv.Delete("foo")
	// if err != nil {
	// 	panic(err)
	// }
}

func unmarshalSessionID(id string) (uint64, *dao.StorageKey, error) {
	i := strings.IndexRune(id, '.')
	if i == -1 {
		return 0, nil, errors.New("fak")
	}

	profileID, err := strconv.ParseUint(id[:i], 36, 64)
	if err != nil {
		return 0, nil, err
	}

	kb, err := base64.RawURLEncoding.DecodeString(id[i+1:])
	if err != nil {
		return 0, nil, err
	}
	storageKey := dao.NewStorageKeyFromBytes(kb)

	return profileID, storageKey, nil
}

func testVPN(bridge js.Value) {
	kv := wasmio.NewKVStore(bridge)
	ds, err := dao.NewMetadataStore(kv)
	if err != nil {
		panic(err)
	}

	profileID := 4303396964
	key := []byte{98, 89, 252, 89, 125, 62, 75, 166, 97, 171, 32, 96, 139, 40, 155, 17, 84, 42, 234, 66, 25, 13, 47, 253, 244, 236, 35, 236, 79, 165, 178, 225}

	_, profileStore, err := ds.LoadSession(uint64(profileID), dao.NewStorageKeyFromBytes(key))
	if err != nil {
		panic(err)
	}

	_ = profileStore

	// h := &vpn.Host{
	// 	Interfaces: []vpn.Interface{
	// 		&wsInterface{
	// 			bridge: bridge,
	// 		},
	// 	},
	// }

	// d := &WebRTCDialer{
	// 	bridge: bridge,
	// }

	// client := vpn.NewClient(profileStore, h, d.DialWebRTC)
	// client.Start()
}

func testWS(bridge js.Value) {
	// broker := vpn.NewBroker()

	// c, err := wasmio.NewWebSocketProxy(bridge, "wss://192.168.0.111:8080/test-bootstrap")
	// if err != nil {
	// 	panic(err)
	// }
	// bc := bufio.NewReadWriter(bufio.NewReader(c), bufio.NewWriterSize(c, 64*1024))

	// broker.InsertAsReceiver(bc, vpn.GenTestKeys())

	// n := 0

	// go func() {
	// 	b := make([]byte, 64*1024)
	// 	for {
	// 		nn, err := p.Read(b)
	// 		if err != nil {
	// 			consoleLog(fmt.Sprintf("read error %s", err))
	// 			return
	// 		}
	// 		p.Write(b[:nn])
	// 		n += nn
	// 	}
	// }()

	// // go func() {
	// // 	<-time.After(30 * time.Second)
	// // 	p.Close()
	// // }()

	// go func() {
	// 	for range time.NewTicker(time.Second).C {
	// 		consoleLog(n * 8 / 1024)
	// 		n = 0
	// 	}
	// }()
}
