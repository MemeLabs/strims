package transcoding

import (
	"fmt"
	"io/ioutil"
	"log"
	"net"
	"net/http"
	"os/exec"
	"time"
	"unsafe"

	"github.com/nareix/joy5/format"
	"github.com/nareix/joy5/format/rtmp"
)

/*
[source -> rtmp server forward -> read rtmp conn in to ffmpeg and output mpeg-ts post -> http server]
- accept conns to rtmp server, forwards data to second rtmp conn
- serve buffer for connections not publishing?
  (forwarding like so https://github.com/nareix/joy5/blob/master/cmd/avtool/forwardrtmp.go#L53-L65)
- using 'ffmpeg -i "rtmp://..."' to read from rtmp server
- output via mpeg-ts post

also possible to output via stdout and udp (TODO: benchmark?)
is reading from rtmp source forward best? (-i "pipe:0"' for stdin)

*/

func runRTMPServer(listenAddr string) error {
	lis, err := net.Listen("tcp", listenAddr)
	if err != nil {
		return err
	}

	s := rtmp.NewServer()
	s.LogEvent = func(c *rtmp.Conn, nc net.Conn, e int) {
		es := rtmp.EventString[e]
		log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), es)
	}

	s.HandleConn = func(c *rtmp.Conn, nc net.Conn) {
		defer nc.Close()

		if !c.Publishing {
			log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), "NotPub")
			return
		}

		q := c.URL.Query()
		fwd := q.Get("fwd")
		if fwd == "" {
			log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), "NoForwardField")
			return
		}
		log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), "fwd", fwd)

		fo := format.URLOpener{}
		w, err := fo.Create(fwd)
		if err != nil {
			log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), "DialFailed")
			return
		}

		c2 := w.Rtmp
		nc2 := w.NetConn
		defer nc2.Close()

		log.Println(unsafe.Pointer(c), nc.LocalAddr(), nc.RemoteAddr(), "DialOK", unsafe.Pointer(c2))

		for {
			pkt, err := c.ReadPacket()
			if err != nil {
				break
			}
			if err = c2.WritePacket(pkt); err != nil {
				break
			}
		}
	}

	for {
		nc, err := lis.Accept()
		if err != nil {
			time.Sleep(time.Second)
			continue
		}
		go s.HandleNetConn(nc)
	}
}

// ffmpegToHTTP takes in a buffer to write to an ffmpeg child process
// that then outputs to an HTTP server.
func ffmpegToHTTP(addr, rtmpServer string) error {

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "unsupported method", http.StatusMethodNotAllowed)
			return
		}

		body, err := ioutil.ReadAll(r.Body)
		if err != nil {
			panic(err)
		}

		fmt.Println(body)
	})
	go http.ListenAndServe(addr, nil)

	if _, err := exec.LookPath("ffmpeg"); err != nil {
		return err
	}

	cmd := exec.Command("ffmpeg", []string{
		// read from rtmpserver, copy as to not reencode
		"-i", rtmpServer, "-c", "copy",
		// cut segment after this, then at duration equal to hls_time
		"-hls_init_time", "2", "-hls_time", "2",
		// max playlist entries, produce playlist with this name
		"-hls_list_size", "6", "-hls_segment_filename", fmt.Sprintf("http://%s/%s", addr, "%d.ts"),
		// EXT-X-PROGRAM-DATE-TIME tags, append new segments go old list removing #EXT-X-ENDLIST, and don't append it, use persistant http connections
		"-hls_flags", "+program_date_time+append_list+omit_endlist", "-http_persistent", "1",
		// ignore io errs, post data, force input or output file format.
		"-ignore_io_errors", "1", "-method", "POST", "-f", "hls", fmt.Sprintf("http://%s/index.m3u8", addr),
	}...)
	if err := cmd.Start(); err != nil {
		return err
	}

	if err := cmd.Wait(); err != nil {
		return err
	}

	return nil
}

func test() {
	rtmpServerAddr := ":9999"
	go func() {
		if err := runRTMPServer(rtmpServerAddr); err != nil {
			panic(err)
		}
	}()

	if err := ffmpegToHTTP(":9998", rtmpServerAddr); err != nil {
		panic(err)
	}
}
