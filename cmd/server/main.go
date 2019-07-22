package main

import (
	"crypto/rand"
	"log"
	"net/http"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/iface"
	"github.com/gogo/protobuf/proto"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
} // use default options

func signal(w http.ResponseWriter, r *http.Request) {
	c, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Print("upgrade:", err)
		return
	}
	log.Println("connection received")

	data := make([]byte, 1024*16)
	if _, err := rand.Read(data); err != nil {
		panic(err)
	}

	defer c.Close()
	for {
		s := iface.Signal{
			Uid:  "test",
			Data: data,
		}
		b, err := proto.Marshal(&s)
		if err != nil {
			log.Println(err)
			return
		}

		if err := c.WriteMessage(websocket.BinaryMessage, b); err != nil {
			log.Println(err)
			return
		}

		time.Sleep(16 * time.Millisecond)

		// mt, message, err := c.ReadMessage()
		// if err != nil {
		// 	log.Println("read:", err)
		// 	break
		// }
		// log.Printf("recv: %s", message)
		// err = c.WriteMessage(mt, message)
		// if err != nil {
		// 	log.Println("write:", err)
		// 	break
		// }
	}
}

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("/signal", signal)

	srv := http.Server{
		Addr:    "0.0.0.0:8082",
		Handler: mux,
	}
	log.Fatal(srv.ListenAndServe())
}
