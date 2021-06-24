package rtmpingress

import (
	"encoding/binary"
	"fmt"
	"io"
	"io/ioutil"
	"net"
	"net/http"
	"os"
	"os/exec"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/gorilla/mux"
	"go.uber.org/zap"
)

const transcoderAddr = "127.0.0.1:0"

// TranscoderMimeType output mime type
const TranscoderMimeType = "video/mp4;codecs=mp4a.40.5,avc1.64001F"

// NewTranscoder ...
func NewTranscoder(logger *zap.Logger) *Transcoder {
	return &Transcoder{
		logger: logger,
	}
}

// Transcoder ...
type Transcoder struct {
	logger  *zap.Logger
	lock    sync.Mutex
	lis     net.Listener
	n       uint32
	writers sync.Map
}

func (h *Transcoder) listen() (err error) {
	router := mux.NewRouter()
	router.HandleFunc("/{key}/{variant}/index.m3u8", postHandleFunc(h.handlePlaylist))
	router.HandleFunc("/{key}/{variant}/init.mp4", postHandleFunc(h.handleInit))
	router.HandleFunc("/{key}/{variant}/{segment:[0-9]+}.m4s", postHandleFunc(h.handleSegment))

	h.lis, err = net.Listen("tcp", transcoderAddr)
	if err != nil {
		return err
	}

	srv := &http.Server{
		Handler: router,
	}
	go func() {
		if err := srv.Serve(h.lis); err != nil {
			h.logger.Debug("failed", zap.Error(err))
		}
	}()

	return nil
}

func (h *Transcoder) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	// noop
	// defer r.Body.Close()
	// io.Copy(ioutil.Discard, r.Body)
}

func (h *Transcoder) handleInit(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)
	twi, ok := h.writers.Load(transcoderKey{params["key"], params["variant"]})
	if !ok {
		http.NotFound(w, r)
		h.logger.Debug("init segment received for unknown stream")
		return
	}

	tw := twi.(*transcoderWriter)

	tw.mu.Lock()
	defer tw.mu.Unlock()

	b, err := ioutil.ReadAll(r.Body)
	if err != nil {
		http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
		h.logger.Debug("error reading init segment", zap.Error(err))
		return
	}

	tw.init = make([]byte, len(b)+2)
	binary.BigEndian.PutUint16(tw.init, uint16(len(b)))
	copy(tw.init[2:], b)

	h.logger.Debug("read init segment", zap.Int("length", len(b)))
}

func (h *Transcoder) handleSegment(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)
	twi, ok := h.writers.Load(transcoderKey{params["key"], params["variant"]})
	if !ok {
		http.NotFound(w, r)
		return
	}

	tw := twi.(*transcoderWriter)

	tw.mu.Lock()
	defer tw.mu.Unlock()

	if _, err := tw.w.Write(tw.init); err != nil {
		http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
		return
	}

	b := pool.Get(32 * 1024)
	defer pool.Put(b)
	if _, err := io.CopyBuffer(tw.w, r.Body, *b); err != nil {
		http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
		return
	}

	if err := tw.w.Flush(); err != nil {
		http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
		return
	}
}

func (h *Transcoder) close(k transcoderKey, cmd *exec.Cmd) {
	h.lock.Lock()
	defer h.lock.Unlock()

	h.n--
	if h.n == 0 {
		h.lis.Close()
	}

	if err := cmd.Process.Kill(); err != nil {
		h.logger.Debug("killing ffmpeg failed", zap.Error(err))
	}

	// wi, _ := h.writers.Load(k)
	// wi.(WriteFlushCloser).Close()

	h.writers.Delete(k)
}

// Transcode ...
func (h *Transcoder) Transcode(srcURI, key, variant string, w WriteFlushCloser) error {
	bin, err := exec.LookPath("ffmpeg")
	if err != nil {
		return err
	}

	h.lock.Lock()
	if h.n == 0 {
		if err := h.listen(); err != nil {
			return err
		}
	}
	h.n++
	h.lock.Unlock()

	k := transcoderKey{key, variant}
	h.writers.Store(k, &transcoderWriter{w: w})

	cmd := exec.Command(bin, buildArgs(srcURI, h.lis.Addr().String(), key, variant)...)
	defer h.close(k, cmd)

	h.logger.Debug("starting ffmpeg", zap.Stringer("cmd", cmd))
	// relayStdio(cmd)

	if err := cmd.Run(); err != nil {
		return err
	}

	return nil
}

type transcoderKey struct {
	key, variant string
}

type transcoderWriter struct {
	mu   sync.Mutex
	init []byte
	w    WriteFlushCloser
}

// WriteFlushCloser ...
type WriteFlushCloser interface {
	Write(p []byte) (int, error)
	Flush() error
}

func postHandleFunc(f http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "unsupported method", http.StatusMethodNotAllowed)
			return
		}

		f(w, r)
	}
}

func relayStdio(cmd *exec.Cmd) error {
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}

	copy := func(w io.Writer, r io.Reader) {
		if _, err := io.Copy(w, r); err != nil {
			panic(err)
		}
	}

	go copy(os.Stderr, stderr)
	go copy(os.Stdout, stdout)
	return nil
}

var transcodingVariants = map[string][]string{
	"source": {"-c", "copy"},
	"720":    {"-vf", "scale=w=1280:h=720:force_original_aspect_ratio=decrease"},
	"480":    {"-vf", "scale=w=854:h=480:force_original_aspect_ratio=decrease"},
	"360":    {"-vf", "scale=w=640:h=360:force_original_aspect_ratio=decrease"},
	"240":    {"-vf", "scale=w=426:h=240:force_original_aspect_ratio=decrease"},
}

func buildArgs(srcURI, addr, key, variant string) []string {
	args := []string{"-i", srcURI}
	args = append(args, transcodingVariants[variant]...)
	args = append(args,
		"-hls_init_time", "1",
		"-hls_time", "1",
		"-hls_segment_type", "fmp4",
		"-hls_fmp4_init_filename", "init.mp4",
		"-hls_segment_filename", fmt.Sprintf("http://%s/%s/%s/%%d.m4s", addr, key, variant),
		"-hls_flags", "+program_date_time+append_list+omit_endlist",
		"-method", "POST",
		"-f", "hls", fmt.Sprintf("http://%s/%s/%s/index.m3u8", addr, key, variant),
	)
	return args
}
