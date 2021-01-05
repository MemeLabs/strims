package rpc

import (
	"context"
	"io/ioutil"
	"net/http"
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/golang/protobuf/proto"
	"go.uber.org/zap"
)

// NewHTTPServer ...
func NewHTTPServer(logger *zap.Logger) *HTTPServer {
	return &HTTPServer{
		ServiceDispatcher: NewServiceDispatcher(logger),
	}
}

// HTTPServer ...
type HTTPServer struct {
	*ServiceDispatcher
}

func (s *HTTPServer) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	b, err := ioutil.ReadAll(r.Body)
	if err != nil {
		httpServeError(http.StatusBadRequest, err, w)
		return
	}

	req := &pb.Call{}
	if err := proto.Unmarshal(b, req); err != nil {
		httpServeError(http.StatusBadRequest, err, w)
		return
	}

	send := func(_ context.Context, res *pb.Call) error {
		return httpServeProto(res, w)
	}
	call := NewCallIn(r.Context(), req, noopParentCallAccessor{}, send)

	s.ServiceDispatcher.Dispatch(call)
}

func httpServeError(statusCode int, err error, w http.ResponseWriter) error {
	w.Header().Set("Content-Type", "text/plain")
	w.Header().Set("Content-Length", strconv.Itoa(len(err.Error())))
	w.WriteHeader(statusCode)
	_, err = w.Write([]byte(err.Error()))
	return err
}

func httpServeProto(m proto.Message, w http.ResponseWriter) error {
	w.Header().Set("Content-Type", "application/protobuf")

	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)
	b.Reset()

	if err := b.EncodeVarint(uint64(proto.Size(m))); err != nil {
		return err
	}
	if err := b.Marshal(m); err != nil {
		return err
	}

	if _, err := w.Write(b.Bytes()); err != nil {
		return err
	}

	if f, ok := w.(http.Flusher); ok {
		f.Flush()
	}

	return nil
}

type noopParentCallAccessor struct{}

func (a noopParentCallAccessor) ParentCallIn() *CallIn {
	return nil
}

func (a noopParentCallAccessor) ParentCallOut() *CallOut {
	return nil
}
