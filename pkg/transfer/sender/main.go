package main

import (
	"log"
	"net"

	"github.com/MemeLabs/go-ppspp/pkg/encoding"
)

func main() {
	log.Println("sender")

	d := &encoding.Datagram{
		ChannelID: 1337,
		Messages: []encoding.Message{
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			&encoding.Request{encoding.NewBin32ChunkAddress(16)},
			encoding.NewData(encoding.Bin(16), make(encoding.Buffer, 1300)),
		},
	}

	// buf := make([]byte, 1500)
	// n := d.Marshal(buf)
	// buf = buf[:n]

	// dd := &encoding.Datagram{}
	// dd.Unmarshal(buf)

	// spew.Dump(dd)

	// // log.Println(buf)
	// log.Println("ppspp", len(buf))

	// s := &iface.Datagram{
	// 	ChannelId: 1337,
	// 	Messages: []*iface.Datagram_Message{
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{DataOneof: &iface.Datagram_Message_Request{Request: &iface.Request{Address: &iface.Bin32ChunkAddress{Value: 16}}}},
	// 		&iface.Datagram_Message{
	// 			DataOneof: &iface.Datagram_Message_Data{
	// 				Data: &iface.Data{
	// 					Address: &iface.Bin32ChunkAddress{
	// 						Value: 16,
	// 					},
	// 					Timestamp: &timestamp.Timestamp{
	// 						Seconds: 1234,
	// 						Nanos:   124,
	// 					},
	// 					Data: make([]byte, 1024),
	// 				},
	// 			},
	// 		},
	// 	},
	// }
	// b, err := proto.Marshal(s)
	// if err != nil {
	// 	log.Println(err)
	// 	return
	// }
	// log.Println("proto", len(b))

	// _ = b

	// _ = d

	go func() {
		// Unlike Dial, ListenPacket creates a connection without any
		// association with peers.
		conn, err := net.ListenPacket("udp", ":1053")
		if err != nil {
			log.Fatal(err)
		}
		defer conn.Close()

		dst, err := net.ResolveUDPAddr("udp", "127.0.0.1:1053")
		if err != nil {
			log.Fatal(err)
		}

		buf := make([]byte, 1500)
		for {
			n := d.Marshal(buf)
			// log.Println(n)
			// buf, err := proto.Marshal(s)
			_, err = conn.WriteTo(buf[:n], dst)
			if err != nil {
				log.Fatal(err)
			}
			// time.Sleep(time.Second)
		}
	}()

	select {}
}
