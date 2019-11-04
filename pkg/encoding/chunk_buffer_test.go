package encoding

import (
	"bufio"
	"fmt"
	"io"
	"log"
	"math/rand"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/davecgh/go-spew/spew"
)

func TestChunkBufferReader_Read(t *testing.T) {
	log.SetFlags(log.LstdFlags | log.Lshortfile)

	b, _ := newChunkBuffer(16)
	b.debug = true
	r := b.Reader()

	_ = r

	v := make([]byte, 1024)
	for i := 0; i < 10; i += 2 {
		b.Set(binmap.Bin(i), v)
	}

	// v[1022] = 8
	// v[1023] = 255
	for i := 0; i < 1024; i++ {
		v[i] = byte(i) & 255
	}
	b.Set(binmap.Bin(10), v)
	b.Set(binmap.Bin(12), v)
	b.Set(binmap.Bin(14), v)

	// spew.Dump(b)

	// go func() {
	// 	t := make([]byte, 100)
	// 	total := 0
	// 	for {
	// 		log.Println("total", total)

	// 		n, err := r.Read(t)
	// 		total += n
	// 		log.Println(t)
	// 		if err != nil {
	// 			log.Println(err)
	// 			return
	// 		}
	// 	}
	// }()

	z, _ := b.Slice(11)

	fmt.Print("\n\n\n\n\n\n")

	spew.Dump(z[0][1024:2048])

	// <-make(chan bool)
}

type chunkStreamWriter struct {
	buf *chunkBuffer
	bin binmap.Bin
}

// Write ...
func (c *chunkStreamWriter) Write(p []byte) (n int, err error) {
	n = len(p)
	if n > ChunkSize {
		n = ChunkSize
	}

	c.buf.Set(c.bin, p[:n])
	c.bin += 2
	return
}

func TestChunkBufferReader_Read2(t *testing.T) {
	log.SetFlags(log.LstdFlags | log.Lshortfile)

	buf, _ := newChunkBuffer(4096)

	bw := bufio.NewWriterSize(&chunkStreamWriter{buf: buf}, ChunkSize)
	cw, _ := chunkstream.NewWriter(bw)

	ready := make(chan struct{})

	go func() {
		close(ready)

		time.Sleep(50 * time.Millisecond)
		br := buf.Reader()
		log.Println("new reader with offset", br.Offset())
		cr, _ := chunkstream.NewReader(br, int64(br.Offset()))

		t := make([]byte, 4096)
		nn := 0
		for {
			n, err := cr.Read(t)
			nn += n
			if err == io.EOF {
				// spew.Dump(t[:n])
				log.Println("got eof", nn)
				nn = 0
			}

			// first := true
			// for i := 0; i < len(t); i++ {
			// 	if t[i] != 255 {
			// 		if first {
			// 			log.Println("---")
			// 			first = false
			// 		}
			// 		log.Println(i)
			// 	}
			// }

			// spew.Dump(t[:n])
			// log.Println("read some bytes...", n, nn)
		}
	}()

	go func() {
		<-ready

		v := make([]byte, 1024)
		for i := 0; i < len(v); i++ {
			v[i] = 255
		}

		time.Sleep(10 * time.Millisecond)

		segmentSize := 50000
		nn := 0
		kn := 0
		// for i := 0; i < 10000; i += 2 {
		for range time.NewTicker(time.Nanosecond).C {
			n := len(v)
			if n+kn > segmentSize {
				n = segmentSize - kn
			}

			n, _ = cw.Write(v[:n])
			nn += n
			kn += n

			if kn == segmentSize {
				cw.Flush()
				kn = 0
				segmentSize = 45000 + rand.Intn(10000)
				// log.Println("flushed", nn)
				// log.Println(buf.bins.String())
			}
		}
	}()

	select {}
}
