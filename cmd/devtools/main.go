// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"path"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	devtoolsv1 "github.com/MemeLabs/strims/pkg/apis/devtools/v1"
	ppsppv1 "github.com/MemeLabs/strims/pkg/apis/devtools/v1/ppspp"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/kv/bbolt"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/ppspptest"
	"github.com/fsnotify/fsnotify"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func main() {
	logger, err := zap.NewDevelopment()
	if err != nil {
		log.Fatalln("logger failed:", err)
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalln("locaing home directory failed:", err)
	}

	store, err := bbolt.NewStore(path.Join(homeDir, ".strims"))
	if err != nil {
		log.Fatalln("opening db failed:", err)
	}

	srv := &devToolsServer{
		logger: logger,
		upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
		},
		devTools: &devToolsService{
			store: store,
		},
		capConn: &capConnService{
			store: store,
		},
	}

	log.Println(srv.Start())
}

type devToolsServer struct {
	logger   *zap.Logger
	upgrader websocket.Upgrader
	devTools *devToolsService
	capConn  *capConnService
}

func (s *devToolsServer) Start() error {
	mux := http.NewServeMux()
	mux.HandleFunc("/api", s.handleAPI)

	srv := http.Server{
		Addr:    "0.0.0.0:8084",
		Handler: mux,
	}
	s.logger.Debug("starting server", zap.String("addr", srv.Addr))
	return srv.ListenAndServe()
}

func (s *devToolsServer) handleAPI(w http.ResponseWriter, r *http.Request) {
	c, err := s.upgrader.Upgrade(w, r, nil)
	if err != nil {
		s.logger.Debug("upgrade failed", zap.Error(err))
		return
	}

	server := rpc.NewServer(s.logger, &rpc.RWDialer{
		Logger:     s.logger,
		ReadWriter: httputil.NewWSReadWriter(c),
	})

	devtoolsv1.RegisterDevToolsService(server, s.devTools)
	ppsppv1.RegisterCapConnService(server, s.capConn)

	server.Listen(r.Context())
}

type devToolsService struct {
	store kv.BlobStore
}

func (s *devToolsService) Test(ctx context.Context, req *devtoolsv1.DevToolsTestRequest) (*devtoolsv1.DevToolsTestResponse, error) {
	return &devtoolsv1.DevToolsTestResponse{
		Message: fmt.Sprintf("hello, %s!", req.Name),
	}, nil
}

type capConnService struct {
	store kv.BlobStore
}

func (s *capConnService) Test(ctx context.Context, req *devtoolsv1.DevToolsTestRequest) (*devtoolsv1.DevToolsTestResponse, error) {
	return &devtoolsv1.DevToolsTestResponse{
		Message: fmt.Sprintf("hello, %s!", req.Name),
	}, nil
}

// WatchLogs ...
func (s *capConnService) WatchLogs(ctx context.Context, req *ppsppv1.CapConnWatchLogsRequest) (<-chan *ppsppv1.CapConnWatchLogsResponse, error) {
	ch := make(chan *ppsppv1.CapConnWatchLogsResponse)

	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, err
	}

	files, err := ioutil.ReadDir(ppspptest.CapConnLogDir())
	if err != nil {
		return nil, err
	}

	go func() {
		defer watcher.Close()

		for _, f := range files {
			if path.Ext(f.Name()) == ppspptest.CapLogExt {
				ch <- &ppsppv1.CapConnWatchLogsResponse{
					Op:   ppsppv1.CapConnWatchLogsResponse_CREATE,
					Name: f.Name(),
				}
			}
		}

	EachEvent:
		for {
			select {
			case event := <-watcher.Events:
				var op ppsppv1.CapConnWatchLogsResponse_Op
				switch {
				case event.Op&fsnotify.Create == fsnotify.Create:
					op = ppsppv1.CapConnWatchLogsResponse_CREATE
				case event.Op&fsnotify.Remove == fsnotify.Remove:
					op = ppsppv1.CapConnWatchLogsResponse_REMOVE
				default:
					continue EachEvent
				}
				if path.Ext(event.Name) == ppspptest.CapLogExt {
					ch <- &ppsppv1.CapConnWatchLogsResponse{
						Op:   op,
						Name: path.Base(event.Name),
					}
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	err = watcher.Add("/tmp/capconn")
	if err != nil {
		return nil, err
	}

	return ch, nil
}

// LoadLog ...
func (s *capConnService) LoadLog(ctx context.Context, req *ppsppv1.CapConnLoadLogRequest) (*ppsppv1.CapConnLoadLogResponse, error) {
	f, err := os.OpenFile(path.Join("/tmp/capconn", req.Name), os.O_RDONLY, 0644)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	hs := []*capLogHandler{}
	err = ppspptest.ReadCapLog(f, func() ppspptest.CapLogHandler {
		rch := &codecHandler{}
		rch.cr = &codec.Reader{
			Handler: rch,
		}

		wch := &codecHandler{}
		wch.cr = &codec.Reader{
			Handler: wch,
		}

		h := &capLogHandler{
			rch: rch,
			rcr: rch.cr,
			wch: wch,
			wcr: wch.cr,
		}
		hs = append(hs, h)
		return h
	})

	res := &ppsppv1.CapConnLoadLogResponse{
		Log: &ppsppv1.CapConnLog{},
	}
	for _, h := range hs {
		res.Log.PeerLogs = append(res.Log.PeerLogs, &ppsppv1.CapConnLog_PeerLog{
			Label:  h.label,
			Events: h.events,
		})
	}

	return res, nil
}

type capLogHandler struct {
	wb     bytes.Buffer
	rch    *codecHandler
	rcr    *codec.Reader
	wch    *codecHandler
	wcr    *codec.Reader
	label  string
	events []*ppsppv1.CapConnLog_PeerLog_Event
}

func (h *capLogHandler) appendEvent(code ppsppv1.CapConnLog_PeerLog_Event_Code, t time.Time, messageTypes []ppsppv1.CapConnLog_PeerLog_Event_MessageType, messageAddresses []uint64) {
	h.events = append(h.events, &ppsppv1.CapConnLog_PeerLog_Event{
		Code:             code,
		Timestamp:        t.UnixNano(),
		MessageTypes:     messageTypes,
		MessageAddresses: messageAddresses,
	})
}

func (h *capLogHandler) HandleInit(t time.Time, label string) {
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_INIT, t, nil, nil)
	h.label = label
}

func (h *capLogHandler) HandleEOF() {}

func (h *capLogHandler) HandleWrite(t time.Time, p []byte) {
	// h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_WRITE, t, nil, nil)
	h.wb.Write(p)
}

func (h *capLogHandler) HandleWriteErr(t time.Time, err error) {
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_WRITE_ERR, t, nil, nil)
}

func (h *capLogHandler) readChannels(b []byte, r io.Reader) error {
	for len(b) != 0 {
		var ch codec.ChannelHeader
		n, err := ch.Unmarshal(b)
		if err != nil {
			return err
		}
		b = b[n:]

		if _, err := r.Read(b[:ch.Length]); err != nil {
			return err
		}
		b = b[ch.Length:]
	}
	return nil
}

func (h *capLogHandler) HandleFlush(t time.Time) {
	h.readChannels(h.wb.Bytes(), h.wcr)

	h.wb.Reset()

	messageTypes, messageAddresses := h.wch.ReadMessages()
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_FLUSH, t, messageTypes, messageAddresses)
}

func (h *capLogHandler) HandleFlushErr(t time.Time, err error) {
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_FLUSH_ERR, t, nil, nil)
}

func (h *capLogHandler) HandleRead(t time.Time, p []byte) {
	h.readChannels(p, h.rcr)

	messageTypes, messageAddresses := h.rch.ReadMessages()
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_READ, t, messageTypes, messageAddresses)
}

func (h *capLogHandler) HandleReadErr(t time.Time, err error) {
	h.appendEvent(ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_READ_ERR, t, nil, nil)
}

type codecHandler struct {
	cr               *codec.Reader
	messageTypes     []ppsppv1.CapConnLog_PeerLog_Event_MessageType
	messageAddresses []uint64
}

func (h *codecHandler) HandleHandshake(v codec.Handshake) error {
	if opt, ok := v.Options.Find(codec.ChunkSizeOption); ok {
		h.cr.ChunkSize = int(opt.(*codec.ChunkSizeProtocolOption).Value)
	}
	if opt, ok := v.Options.Find(codec.MerkleHashTreeFunctionOption); ok {
		h.cr.IntegrityHashSize = integrity.MerkleHashTreeFunction(opt.(*codec.MerkleHashTreeFunctionProtocolOption).Value).HashSize()
	}
	if opt, ok := v.Options.Find(codec.LiveSignatureAlgorithmOption); ok {
		h.cr.IntegritySignatureSize = integrity.LiveSignatureAlgorithm(opt.(*codec.LiveSignatureAlgorithmProtocolOption).Value).SignatureSize()
	}
	return nil
}

func (h *codecHandler) appendMessage(t ppsppv1.CapConnLog_PeerLog_Event_MessageType, a uint64) {
	h.messageTypes = append(h.messageTypes, t)
	h.messageAddresses = append(h.messageAddresses, a)
}

func (h *codecHandler) ReadMessages() ([]ppsppv1.CapConnLog_PeerLog_Event_MessageType, []uint64) {
	t := h.messageTypes
	a := h.messageAddresses
	h.messageTypes = nil
	h.messageAddresses = nil
	return t, a
}

func (h *codecHandler) HandleData(v codec.Data) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_DATA, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleAck(v codec.Ack) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_ACK, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleHave(v codec.Have) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_HAVE, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleIntegrity(v codec.Integrity) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_INTEGRITY, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleSignedIntegrity(v codec.SignedIntegrity) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_SIGNED_INTEGRITY, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleRequest(v codec.Request) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_REQUEST, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleCancel(v codec.Cancel) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_CANCEL, uint64(v.Address))
	return nil
}

func (h *codecHandler) HandleChoke(v codec.Choke) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_CHOKE, 0)
	return nil
}

func (h *codecHandler) HandleUnchoke(v codec.Unchoke) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_UNCHOKE, 0)
	return nil
}

func (h *codecHandler) HandlePing(v codec.Ping) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_PING, v.Nonce.Value)
	return nil
}

func (h *codecHandler) HandlePong(v codec.Pong) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_PONG, v.Nonce.Value)
	return nil
}

func (h *codecHandler) HandleStreamRequest(v codec.StreamRequest) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_STREAM_REQUEST, uint64(v.Stream))
	return nil
}

func (h *codecHandler) HandleStreamCancel(v codec.StreamCancel) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_STREAM_CANCEL, uint64(v.Stream))
	return nil
}

func (h *codecHandler) HandleStreamOpen(v codec.StreamOpen) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_STREAM_OPEN, uint64(v.Stream))
	return nil
}

func (h *codecHandler) HandleStreamClose(v codec.StreamClose) error {
	h.appendMessage(ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_STREAM_CLOSE, uint64(v.Stream))
	return nil
}
