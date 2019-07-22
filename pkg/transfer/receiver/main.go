package main

import (
	"log"
	"net"
	"net/http"
	_ "net/http/pprof"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

func main() {
	log.Println("receiver")

	var n uint64

	go func() {
		for {
			log.Println(atomic.SwapUint64(&n, 0))
			time.Sleep(time.Second)
		}
	}()

	go func() {
		// listen to incoming udp packets
		pc, err := net.ListenPacket("udp", ":1053")
		if err != nil {
			log.Fatal(err)
		}
		defer pc.Close()

		buf := make([]byte, 1500)
		for {
			size, addr, err := pc.ReadFrom(buf)
			if err != nil {
				log.Println(err)
				// continue
			}
			_ = size
			_ = addr

			// s := &iface.Datagram{}
			// proto.Unmarshal(buf, s)

			dd := &encoding.Datagram{}
			dd.Unmarshal(buf)

			// log.Printf("received %d bytes from %v", size, addr)

			// 	go serve(pc, addr, buf[:n])
			atomic.AddUint64(&n, uint64(1200))
		}
	}()

	go func() {
		h := encoding.NewHost(encoding.NewDefaultHostOptions())
		h.Run()
	}()

	go func() {
		log.Println(http.ListenAndServe("localhost:6060", nil))
	}()

	select {}
}

func serve(pc net.PacketConn, addr net.Addr, buf []byte) {
	// 0 - 1: ID
	// 2: QR(1): Opcode(4)
	buf[2] |= 0x80 // Set QR bit

	pc.WriteTo(buf, addr)
}
