package main

import (
	"bytes"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/lhls"
	"github.com/gorilla/mux"
	"github.com/nareix/joy4/av/avutil"
	"github.com/nareix/joy4/av/pubsub"
	"github.com/nareix/joy4/format"
	"github.com/nareix/joy4/format/rtmp"
)

func init() {
	format.RegisterAll()
}

func main() {
	server := &rtmp.Server{}

	l := &sync.RWMutex{}
	type Channel struct {
		queue  *pubsub.Queue
		stream *lhls.Stream
	}
	channels := map[string]*Channel{}

	server.HandlePublish = func(conn *rtmp.Conn) {
		name, _ := rtmp.SplitPath(conn.URL)
		streams, _ := conn.Streams()

		log.Println("new rtmp stream", name)

		l.Lock()
		ch, ok := channels[name]
		if !ok {
			ch = &Channel{
				queue:  pubsub.NewQueue(),
				stream: lhls.NewDefaultStream(),
			}
			ch.queue.WriteHeader(streams)
			channels[name] = ch
		}
		l.Unlock()
		if ch == nil {
			return
		}

		go func() {
			ch.stream.WriteHeader(streams)
			err := ch.stream.CopyPackets(ch.queue.Oldest())
			log.Printf("stream %s closed %v", name, err)
		}()

		avutil.CopyPackets(ch.queue, conn)

		l.Lock()
		delete(channels, name)
		l.Unlock()
		ch.queue.Close()
	}

	router := mux.NewRouter()
	http.Handle("/", router)

	router.HandleFunc("/hls/{name:[a-z0-9]+}/{segment:[0-9]{0,9}}.ts", func(w http.ResponseWriter, r *http.Request) {
		params := mux.Vars(r)

		l.RLock()
		ch := channels[params["name"]]
		l.RUnlock()

		if ch != nil {
			w.Header().Set("Content-Type", "video/mp2t")
			w.Header().Set("Transfer-Encoding", "chunked")
			w.Header().Set("Access-Control-Allow-Origin", "*")
			w.WriteHeader(200)

			segment, _ := strconv.ParseUint(params["segment"], 10, 64)
			cr, err := ch.stream.SegmentReader(segment)
			if err != nil {
				log.Println("reader not found")
				http.NotFound(w, r)
				return
			}

			io.Copy(w, cr)

			// flusher := w.(http.Flusher)
			// flusher.Flush()

		} else {
			log.Println("channel not found")
			http.NotFound(w, r)
		}
	})

	router.HandleFunc("/hls/{name:[a-z0-9]+}/index.m3u8", func(w http.ResponseWriter, r *http.Request) {
		params := mux.Vars(r)

		l.RLock()
		ch := channels[params["name"]]
		l.RUnlock()

		if ch != nil {
			w.Header().Set("Content-Type", "application/vnd.apple.mpegurl")
			w.Header().Set("Access-Control-Allow-Origin", "*")
			w.WriteHeader(200)

			low, high := ch.stream.Range()

			var b bytes.Buffer
			b.WriteString("#EXTM3U\n")
			b.WriteString("#EXT-X-VERSION:3\n")
			b.WriteString(fmt.Sprintf("#EXT-X-MEDIA-SEQUENCE:%d\n", high))
			b.WriteString("#EXT-X-TARGETDURATION:1\n")

			for i := low; i <= high; i++ {
				b.WriteString(fmt.Sprintf("#EXTINF:1.000,%s\n", params["name"]))
				b.WriteString(fmt.Sprintf("/hls/%s/%d.ts\n", params["name"], i))
			}

			io.Copy(w, &b)
		} else {
			http.NotFound(w, r)
		}
	})

	go http.ListenAndServe(":8089", nil)

	if err := server.ListenAndServe(); err != nil {
		log.Panic(err)
	}

	// ffmpeg -re -i movie.flv -c copy -f flv rtmp://localhost/movie
	// ffmpeg -f avfoundation -i "0:0" .... -f flv rtmp://localhost/screen
	// ffplay http://localhost:8089/hls/movie/index.m3u8
	// ffplay http://localhost:8089/hls/screen/index.m3u8
}
