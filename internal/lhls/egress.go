package lhls

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"sync"

	"github.com/gorilla/mux"
)

// Channel ...
type Channel struct {
	Stream *Stream
}

// NewEgress ...
func NewEgress() (s *Egress) {
	s = &Egress{
		channels: map[string]*Channel{},
	}
	s.Server = http.Server{
		Addr:    ":8089",
		Handler: s.httpHandler(),
	}
	return
}

// Egress ...
type Egress struct {
	mu sync.RWMutex
	http.Server
	channels map[string]*Channel
}

// AddChannel ...
func (s *Egress) AddChannel(c *Channel) {
	s.mu.Lock()
	s.channels["test"] = c
	s.mu.Unlock()
}

// RemoveChannel ...
func (s *Egress) RemoveChannel(c *Channel) {
	s.mu.Lock()
	delete(s.channels, "test")
	s.mu.Unlock()
}

func (s *Egress) httpHandler() http.Handler {
	r := mux.NewRouter()
	r.HandleFunc("/hls/{name:[a-z0-9]+}/{segment:[0-9]{0,9}}.ts", s.handleSegment)
	r.HandleFunc("/hls/{name:[a-z0-9]+}/index.m3u8", s.handlePlaylist)
	return r
}

func (s *Egress) handleSegment(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch == nil {
		http.NotFound(w, r)
		return
	}

	segment, err := strconv.ParseUint(params["segment"], 10, 64)
	if err != nil {
		http.NotFound(w, r)
		return
	}

	cr, err := ch.Stream.SegmentReader(segment)
	if err != nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "video/mp2t")
	w.Header().Set("Transfer-Encoding", "chunked")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.WriteHeader(200)

	// TODO: write deadline to prevent reading from recycled segments

	io.Copy(w, cr)
}

func (s *Egress) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch == nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "application/vnd.apple.mpegurl")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.WriteHeader(200)

	// TODO: low segment reads may exit early if the buffer gets recycled before
	// all the reads finish... truncate the playlist to avoid advertising
	// segments that expire soon
	low, high := ch.Stream.Range()

	var b bytes.Buffer
	b.WriteString("#EXTM3U\n")
	b.WriteString("#EXT-X-VERSION:3\n")
	b.WriteString(fmt.Sprintf("#EXT-X-MEDIA-SEQUENCE:%s\n", strconv.FormatUint(high, 10)))
	b.WriteString("#EXT-X-TARGETDURATION:1\n")

	for i := low; i <= high; i++ {
		b.WriteString(fmt.Sprintf("#EXTINF:1.000,%s\n", params["name"]))
		b.WriteString(fmt.Sprintf("/hls/%s/%s.ts\n", params["name"], strconv.FormatUint(i, 10)))
	}

	io.Copy(w, &b)
}
