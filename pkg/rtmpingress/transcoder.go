package rtmpingress

import (
	"fmt"
	"io"
	"io/ioutil"
	"net"
	"net/http"
	"os"
	"os/exec"
	"sync"

	"github.com/gorilla/mux"
)

// Transcoder ...
type Transcoder struct {
	lock    sync.Mutex
	lis     net.Listener
	n       uint32
	writers sync.Map
}

func (h *Transcoder) listen() (err error) {
	router := mux.NewRouter()
	router.HandleFunc("/{key}/{variant}/index.m3u8", postHandleFunc(h.handlePlaylist))
	router.HandleFunc("/{key}/{variant}/{segment:[0-9]+}.ts", postHandleFunc(h.handleSegment))

	h.lis, err = net.Listen("tcp", ":0")
	if err != nil {
		return err
	}

	srv := &http.Server{
		Handler: router,
	}
	go srv.Serve(h.lis)

	return nil
}

func (h *Transcoder) handlePlaylist(w http.ResponseWriter, r *http.Request) {
	io.Copy(ioutil.Discard, r.Body)
	r.Body.Close()
}

func (h *Transcoder) handleSegment(w http.ResponseWriter, r *http.Request) {
	params := mux.Vars(r)
	wi, ok := h.writers.Load(transcoderKey{params["key"], params["variant"]})
	if !ok {
		http.NotFound(w, r)
		return
	}

	defer r.Body.Close()
	if _, err := io.Copy(wi.(WriteFlushCloser), r.Body); err != nil {
		http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
		return
	}

	if err := wi.(WriteFlushCloser).Flush(); err != nil {
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

	cmd.Process.Kill()

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
		h.listen()
	}
	h.n++
	h.lock.Unlock()

	k := transcoderKey{key, variant}
	h.writers.Store(k, w)

	cmd := exec.Command(bin, buildArgs(srcURI, h.lis.Addr().String(), key, variant)...)
	defer h.close(k, cmd)

	if err := cmd.Run(); err != nil {
		return err
	}

	return nil
}

type transcoderKey struct {
	key, variant string
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
	go io.Copy(os.Stderr, stderr)
	go io.Copy(os.Stdout, stdout)
	return nil
}

var transcodingVariants = map[string][]string{
	"source": {"-c", "copy"},
	"720":    {"-vf", "scale=w=1280:h=720:force_original_aspect_ratio=decrease"},
	"480":    {"-vf", "scale=w=858:h=480:force_original_aspect_ratio=decrease"},
	"360":    {"-vf", "scale=w=480:h=360:force_original_aspect_ratio=decrease"},
	"240":    {"-vf", "scale=w=352:h=240:force_original_aspect_ratio=decrease"},
}

func buildArgs(srcURI, addr, key, variant string) []string {
	args := []string{"-i", srcURI}
	args = append(args, transcodingVariants[variant]...)
	args = append(args,
		"-hls_init_time", "2",
		"-hls_time", "2",
		"-hls_segment_filename", fmt.Sprintf("http://%s/%s/%s/%%d.ts", addr, key, variant),
		"-hls_flags", "+program_date_time+append_list+omit_endlist",
		"-method", "POST",
		"-f", "hls", fmt.Sprintf("http://%s/%s/%s/index.m3u8", addr, key, variant),
	)
	return args
}
