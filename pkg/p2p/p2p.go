package p2p

import (
	"fmt"
	"log"
	"net"
	"net/url"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/iface"
	"github.com/gogo/protobuf/proto"
	"github.com/gorilla/websocket"
	"github.com/pion/webrtc/v2"
)

type Client struct{}

type Signal interface {
}

type SignallingChannel interface {
	Recv() (*Signal, error)
	Send(sig *Signal) error
}

func NewRTCConn() {
	config := webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{"stun:stun.l.google.com:19302"},
			},
		},
	}

	// Create a new RTCPeerConnection
	peerConnection, err := webrtc.NewPeerConnection(config)
	if err != nil {
		panic(err)
	}

	// Create a datachannel with label 'data'
	dataChannel, err := peerConnection.CreateDataChannel("data", nil)
	if err != nil {
		panic(err)
	}

	// Set the handler for ICE connection state
	// This will notify you when the peer has connected/disconnected
	peerConnection.OnICEConnectionStateChange(func(connectionState webrtc.ICEConnectionState) {
		fmt.Printf("ICE Connection State has changed: %s\n", connectionState.String())
	})

	// Register channel opening handling
	dataChannel.OnOpen(func() {
		fmt.Printf("Data channel '%s'-'%d' open. Random messages will now be sent to any connected DataChannels every 5 seconds\n", dataChannel.Label(), dataChannel.ID())

		for range time.NewTicker(5 * time.Second).C {
			message := "some text"
			fmt.Printf("Sending '%s'\n", message)

			// Send the message as text
			sendErr := dataChannel.SendText(message)
			if sendErr != nil {
				panic(sendErr)
			}
		}
	})

	// Register text message handling
	dataChannel.OnMessage(func(msg webrtc.DataChannelMessage) {
		fmt.Printf("Message from DataChannel '%s': '%s'\n", dataChannel.Label(), string(msg.Data))
	})

	// Create an offer to send to the browser
	offer, err := peerConnection.CreateOffer(nil)
	if err != nil {
		panic(err)
	}

	// Sets the LocalDescription, and starts our UDP listeners
	err = peerConnection.SetLocalDescription(offer)
	if err != nil {
		panic(err)
	}

	// Output the offer in base64 so we can paste it in browser
	fmt.Println(offer)

	// Wait for the answer to be pasted
	answer := webrtc.SessionDescription{}
	// signal.Decode(signal.MustReadStdin(), &answer)

	// Apply the answer as the remote description
	err = peerConnection.SetRemoteDescription(answer)
	if err != nil {
		panic(err)
	}
}

func NewWS() {
	addr := "192.168.0.111:8082"

	u := url.URL{Scheme: "ws", Host: addr, Path: "/signal"}
	log.Printf("connecting to %s", u.String())

	c, _, err := websocket.DefaultDialer.Dial(u.String(), nil)
	if err != nil {
		log.Fatal("dial:", err)
	}
	defer c.Close()

	go func() {
		for {
			_, data, err := c.ReadMessage()
			if err != nil {
				log.Println("read:", err)
				return
			}
			// log.Printf("recv: %d", len(data))
			// h := sha256.New()
			// h.Write(data)

			s := iface.Signal{}
			if err := proto.Unmarshal(data, &s); err != nil {
				log.Println(err)
				return
			}

			log.Println(s.Uid, len(s.Data))
		}
	}()

	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()

	for {
		err := c.WriteMessage(websocket.TextMessage, []byte("test"))
		if err != nil {
			log.Println("write:", err)
			return
		}

		time.Sleep(time.Second)
	}

	// ws := js.Global().Get("WebSocket").New("ws://192.168.0.111:8082/signal")
	// ws.Set("binaryType", "arraybuffer")

	// ws.Call("addEventListener", "open", js.FuncOf(func(this js.Value, args []js.Value) interface{} {
	// 	log.Println("open")

	// 	// ws.Call("send", js.TypedArrayOf([]byte{123}))
	// 	return nil
	// }))

	// ws.Call("addEventListener", "message", js.FuncOf(func(this js.Value, args []js.Value) interface{} {
	// 	data := uint8ArrayValueToBytes(js.Global().Get("Uint8Array").New(args[0].Get("data")))

	// 	h := sha256.New()
	// 	h.Write(data)

	// 	// s := iface.Signal{}
	// 	// if err := proto.Unmarshal(data, &s); err != nil {
	// 	// 	log.Println(err)
	// 	// 	return nil
	// 	// }

	// 	// log.Println(s.Uid, len(s.Data))

	// 	return nil
	// }))

	// ws.Call("addEventListener", "close", js.FuncOf(func(this js.Value, args []js.Value) interface{} {
	// 	log.Println("close")

	// 	return nil
	// }))
}

type WSSignallingChannel struct {
	conn *net.Conn
}

func NewClient() *Client {
	NewWS()

	return &Client{}
}
