package httputil

import (
	"net/http"

	"github.com/gorilla/websocket"
)

var DefaultUpgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type WSHandlerFunc func(c *websocket.Conn)

func (h WSHandlerFunc) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	c, err := DefaultUpgrader.Upgrade(w, r, nil)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	h(c)
}
