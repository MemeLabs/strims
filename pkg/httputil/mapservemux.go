// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package httputil

import (
	"net/http"
	"sort"
	"strings"
	"sync"
)

func NewMapServeMux() *MapServeMux {
	return &MapServeMux{
		routes: map[string]http.Handler{},
	}
}

type prefixRoute struct {
	prefix  string
	handler http.Handler
}

type MapServeMux struct {
	lock     sync.Mutex
	routes   map[string]http.Handler
	prefixes []prefixRoute
}

func (s *MapServeMux) Handler(r *http.Request) http.Handler {
	s.lock.Lock()
	defer s.lock.Unlock()

	h, ok := s.routes[r.URL.Path]
	if ok {
		return h
	}

	for _, p := range s.prefixes {
		if strings.HasPrefix(r.URL.Path, p.prefix) {
			return p.handler
		}
	}

	return http.NotFoundHandler()
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
	s.lock.Lock()
	defer s.lock.Unlock()

	if !strings.HasSuffix(path, "*") {
		s.routes[path] = h
		return
	}

	prefix := strings.TrimSuffix(path, "*")
	s.prefixes = append(s.prefixes, prefixRoute{prefix, h})
	sort.Slice(s.prefixes, func(i, j int) bool { return len(s.prefixes[i].prefix) > len(s.prefixes[j].prefix) })
}

func (s *MapServeMux) StopHandling(path string) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if !strings.HasSuffix(path, "*") {
		delete(s.routes, path)
		return
	}

	prefix := strings.TrimSuffix(path, "*")
	for i, p := range s.prefixes {
		if p.prefix == prefix {
			copy(s.prefixes[i:], s.prefixes[i+1:])
			l := len(s.prefixes) - 1
			s.prefixes[l] = prefixRoute{}
			s.prefixes = s.prefixes[:l]
		}
	}
}
