package encoding

import (
	"fmt"
	"log"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
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
