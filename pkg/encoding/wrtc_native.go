// +build !js

package encoding

import (
	"context"
	"log"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/gorilla/websocket"
	"github.com/pion/webrtc/v2"
)

// nativeWRTCConn ...
type nativeWRTCConn struct {
	a  *NativeWRTCAdapter
	pc *webrtc.PeerConnection
	dc *webrtc.DataChannel
}

func (j nativeWRTCConn) isWRTCConn() {}

type nativeWRTCData struct {
	c nativeWRTCConn
	d []byte
}

// NativeWRTCAdapter ...
type NativeWRTCAdapter struct {
	SignalAddress string
	s             *http.Server
	ctx           context.Context
	data          chan nativeWRTCData
}

func (a *NativeWRTCAdapter) listen(ctx context.Context) error {
	// TODO: if we're going to do init stuff here maybe name it something else...

	a.ctx = ctx
	a.data = make(chan nativeWRTCData, 16)

	if a.SignalAddress == "" {
		return nil
	}

	u := websocket.Upgrader{
		CheckOrigin: func(r *http.Request) bool {
			// TODO: allowed origins...
			return true
		},
	}

	h := mux.NewRouter()
	h.HandleFunc("/{id}", func(w http.ResponseWriter, r *http.Request) {
		c, err := u.Upgrade(w, r, nil)
		if err != nil {
			log.Print("upgrade:", err)
			return
		}
		log.Println("connection received")

		a.newThing(c)
	})

	a.s = &http.Server{
		Addr:    a.SignalAddress,
		Handler: h,
	}
	log.Println("wrtc signal server listening at", a.SignalAddress)

	go a.s.ListenAndServe()

	return nil
}

func (a *NativeWRTCAdapter) close() (err error) {
	s := a.s
	if a.s == nil {
		return nil
	}
	err = s.Close()
	a.s = nil

	close(a.data)

	return
}

func newBool(v bool) *bool {
	return &v
}

func (a *NativeWRTCAdapter) newThing(sc *websocket.Conn) (err error) {
	config := webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{"stun:stun.l.google.com:19302"},
			},
		},
	}

	c := nativeWRTCConn{
		a: a,
	}

	c.pc, err = webrtc.NewPeerConnection(config)
	if err != nil {
		return
	}

	c.dc, err = c.pc.CreateDataChannel("data", &webrtc.DataChannelInit{
		Ordered: newBool(false),
	})
	if err != nil {
		return
	}

	c.dc.OnOpen(func() {
		log.Println("on open...")
	})

	c.dc.OnMessage(func(msg webrtc.DataChannelMessage) {
		a.data <- nativeWRTCData{
			c: c,
			d: msg.Data,
		}
	})

	c.pc.OnICEConnectionStateChange(func(s webrtc.ICEConnectionState) {
		log.Println("connection state changed to", s.String())
		if s == webrtc.ICEConnectionStateClosed {
			a.handleClose()
		}
	})

	offer, err := c.pc.CreateOffer(nil)
	if err != nil {
		return
	}

	err = c.pc.SetLocalDescription(offer)
	if err != nil {
		return
	}

	sc.WriteJSON(offer)

	answer := webrtc.SessionDescription{}
	if err = sc.ReadJSON(&answer); err != nil {
		return
	}

	err = c.pc.SetRemoteDescription(answer)
	return
}

func (a *NativeWRTCAdapter) handleClose() interface{} {
	// is there cleanup to do here...?

	return nil
}

func (a *NativeWRTCAdapter) dial(uri TransportURI) (tc wrtcConn, err error) {
	return
}

func (a *NativeWRTCAdapter) write(b []byte, c wrtcConn) (err error) {
	return c.(nativeWRTCConn).dc.Send(b)
}

func (a *NativeWRTCAdapter) read(b []byte) (n int, c wrtcConn, err error) {
	select {
	case d := <-a.data:
		n = copy(b, d.d)
		c = d.c
	case <-a.ctx.Done():
		err = a.ctx.Err()
	}
	return
}

func (a *NativeWRTCAdapter) closeConn(c wrtcConn) (err error) {
	return c.(nativeWRTCConn).pc.Close()
}
