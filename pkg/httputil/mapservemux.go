package httputil

import (
	"net/http"

	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
)

func NewMapServeMux() *MapServeMux {
	return &MapServeMux{}
}

type MapServeMux struct {
	routes syncutil.Map[string, http.Handler]
}

func (s *MapServeMux) Handler(r *http.Request) http.Handler {
	h, ok := s.routes.Get(r.URL.Path)
	if !ok {
		return http.NotFoundHandler()
	}
	return h
}

func (s *MapServeMux) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.Handler(r).ServeHTTP(w, r)
}

func (s *MapServeMux) HandleFunc(path string, h http.HandlerFunc) {
	s.Handle(path, h)
}

func (s *MapServeMux) HandleWSFunc(path string, h WSHandlerFunc) {
	s.Handle(path, h)
}

func (s *MapServeMux) Handle(path string, h http.Handler) {
	s.routes.Set(path, h)
}

func (s *MapServeMux) StopHandling(path string) {
	s.routes.Delete(path)
}
