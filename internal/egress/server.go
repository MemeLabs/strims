package egress

import (
	"bytes"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
	"github.com/gorilla/mux"
	"github.com/nareix/joy4/format"
)

func init() {
	format.RegisterAll()
}

// Channel ...
type Channel struct {
	Stream *lhls.Stream
}

// New ...
func New() (s *Server) {
	s = &Server{
		channels: map[string]*Channel{},
	}
	s.Server = http.Server{
		Addr:    ":8089",
		Handler: s.httpHandler(),
	}
	return
}

// Server ...
type Server struct {
	mu sync.RWMutex
	http.Server
	channels map[string]*Channel
}

// AddChannel ...
func (s *Server) AddChannel(c *Channel) {
	s.mu.Lock()
	s.channels["test"] = c
	s.mu.Unlock()
}

// RemoveChannel ...
func (s *Server) RemoveChannel(c *Channel) {
	s.mu.Lock()
	delete(s.channels, "test")
	s.mu.Unlock()
}

func (s *Server) httpHandler() http.Handler {
	r := mux.NewRouter()
	r.HandleFunc("/hls/{name:[a-z0-9]+}/{segment:[0-9]{0,9}}.ts", s.handleSegment)
	r.HandleFunc("/hls/{name:[a-z0-9]+}/index.m3u8", s.handlePlaylist)
	return r
}

func (s *Server) handleSegment(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch != nil {
		segment, _ := strconv.Atoi(params["segment"])
		cr, err := ch.Stream.SegmentReader(segment)
		if err != nil {
			log.Println("reader not found")
			http.NotFound(w, r)
			return
		}

		w.Header().Set("Content-Type", "video/mp2t")
		w.Header().Set("Transfer-Encoding", "chunked")
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.WriteHeader(200)

		io.Copy(w, cr)

		// flusher := w.(http.Flusher)
		// flusher.Flush()

	} else {
		log.Println("channel not found")
		http.NotFound(w, r)
	}
}

func (s *Server) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch != nil {
		w.Header().Set("Content-Type", "application/vnd.apple.mpegurl")
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.WriteHeader(200)

		low, high := ch.Stream.Range()

		var b bytes.Buffer
		b.WriteString("#EXTM3U\n")
		b.WriteString("#EXT-X-VERSION:3\n")
		b.WriteString(fmt.Sprintf("#EXT-X-MEDIA-SEQUENCE:%d\n", high))
		b.WriteString("#EXT-X-TARGETDURATION:1\n")

		for i := low; i <= high; i++ {
			b.WriteString(fmt.Sprintf("#EXTINF:1.000,%s\n", params["name"]))
			b.WriteString(fmt.Sprintf("/hls/%s/%d.ts\n", params["name"], i))
		}

		io.Copy(w, &b)
	} else {
		http.NotFound(w, r)
	}
}
