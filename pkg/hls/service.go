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

// NewService ...
func NewService(prefix string) (s *Service) {
	return &Service{
		prefix:  prefix,
		streams: map[string]*Stream{},
	}
}

// Service ...
type Service struct {
	prefix  string
	mu      sync.RWMutex
	streams map[string]*Stream
}

// InsertStream ...
func (s *Service) InsertStream(name string, stream *Stream) bool {
	s.mu.Lock()
	defer s.mu.Unlock()

	ok := s.streams[name] == nil
	if ok {
		s.streams[name] = stream
	}
	return ok
}

// RemoveStream ...
func (s *Service) RemoveStream(name string) {
	s.mu.Lock()
	defer s.mu.Unlock()

	delete(s.streams, name)
}

func (s *Service) PlaylistRoute(name string) string {
	return fmt.Sprintf("%s/%s/index.m3u8", s.prefix, name)
}

// Handler ...
func (s *Service) Handler() http.Handler {
	r := mux.NewRouter()
	r.HandleFunc(s.prefix+"/{name:[a-z0-9]+}/{segment:[0-9]{0,9}}.m4s", s.handleSegment)
	r.HandleFunc(s.prefix+"/{name:[a-z0-9]+}/init.mp4", s.handleInit)
	r.HandleFunc(s.prefix+"/{name:[a-z0-9]+}/index.m3u8", s.handlePlaylist)
	return r
}

func (s *Service) handleSegment(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	stream := s.streams[params["name"]]
	s.mu.RUnlock()

	if stream == nil {
		http.NotFound(w, r)
		return
	}

	segment, err := strconv.ParseUint(params["segment"], 10, 64)
	if err != nil {
		http.NotFound(w, r)
		return
	}

	cr, err := stream.SegmentReader(segment)
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
	stream := s.streams[params["name"]]
	s.mu.RUnlock()

	if stream == nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "video/mp4")
	w.WriteHeader(200)
	io.Copy(w, stream.InitReader())
}

func (s *Service) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)

	s.mu.RLock()
	stream := s.streams[params["name"]]
	s.mu.RUnlock()

	if stream == nil {
		http.NotFound(w, r)
		return
	}

	w.Header().Set("Content-Type", "application/vnd.apple.mpegurl")
	w.WriteHeader(200)

	// TODO: low segment reads may exit early if the buffer gets recycled before
	// all the reads finish... truncate the playlist to avoid advertising
	// segments that expire soon
	low, high, dm := stream.Range()

	var b bytes.Buffer
	b.WriteString("#EXTM3U\n")
	b.WriteString("#EXT-X-VERSION:7\n")
	b.WriteString("#EXT-X-TARGETDURATION:1\n")
	b.WriteString(fmt.Sprintf("#EXT-X-MEDIA-SEQUENCE:%s\n", strconv.FormatUint(low, 10)))
	b.WriteString(fmt.Sprintf("#EXT-X-MAP:URI=\"%s/%s/init.mp4\"\n", s.prefix, params["name"]))

	for i := low; i < high; i++ {
		if dm.Get(i, high) {
			b.WriteString("#EXT-X-DISCONTINUITY\n")
		} else {
			b.WriteString(fmt.Sprintf("#EXTINF:1,\n"))
			b.WriteString(fmt.Sprintf("%s/%s/%s.m4s\n", s.prefix, params["name"], strconv.FormatUint(i, 10)))
		}
	}

	io.Copy(w, &b)
}
