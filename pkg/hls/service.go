package hls

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
	Name   string
	Stream *Stream
}

// NewService ...
func NewService() (s *Service) {
	return &Service{
		channels: map[string]*Channel{},
	}
}

// Service ...
type Service struct {
	mu       sync.RWMutex
	channels map[string]*Channel
}

// InsertChannel ...
func (s *Service) InsertChannel(c *Channel) {
	s.mu.Lock()
	s.channels[c.Name] = c
	s.mu.Unlock()
}

// RemoveChannel ...
func (s *Service) RemoveChannel(c *Channel) {
	s.mu.Lock()
	delete(s.channels, c.Name)
	s.mu.Unlock()
}

// Handler ...
func (s *Service) Handler() http.Handler {
	r := mux.NewRouter()
	r.HandleFunc("/hls/{name:[a-z0-9]+}/{segment:[0-9]{0,9}}.m4s", s.handleSegment)
	r.HandleFunc("/hls/{name:[a-z0-9]+}/init.mp4", s.handleInit)
	r.HandleFunc("/hls/{name:[a-z0-9]+}/index.m3u8", s.handlePlaylist)
	return r
}

func (s *Service) handleSegment(w http.ResponseWriter, r *http.Request) {
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

	w.Header().Set("Content-Type", "video/iso.segment")
	w.WriteHeader(200)

	io.Copy(w, cr)
}

func (s *Service) handleInit(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch == nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "video/mp4")
	w.WriteHeader(200)
	io.Copy(w, ch.Stream.InitReader())
}

func (s *Service) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	ch := s.channels[params["name"]]
	s.mu.RUnlock()

	if ch == nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "application/vnd.apple.mpegurl")
	w.WriteHeader(200)

	// TODO: low segment reads may exit early if the buffer gets recycled before
	// all the reads finish... truncate the playlist to avoid advertising
	// segments that expire soon
	low, high := ch.Stream.Range()

	var b bytes.Buffer
	b.WriteString("#EXTM3U\n")
	b.WriteString("#EXT-X-VERSION:7\n")
	b.WriteString("#EXT-X-TARGETDURATION:1\n")
	b.WriteString(fmt.Sprintf("#EXT-X-MEDIA-SEQUENCE:%s\n", strconv.FormatUint(low, 10)))
	b.WriteString(fmt.Sprintf("#EXT-X-MAP:URI=\"/hls/%s/init.mp4\"\n", params["name"]))

	for i := low; i < high; i++ {
		b.WriteString(fmt.Sprintf("#EXTINF:1,\n"))
		b.WriteString(fmt.Sprintf("/hls/%s/%s.m4s\n", params["name"], strconv.FormatUint(i, 10)))
	}

	io.Copy(w, &b)
}