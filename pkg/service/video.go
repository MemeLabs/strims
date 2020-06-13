package service

import (
	"bytes"
	"encoding/json"
	"io"
	"log"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// NewVideoThing ...
func NewVideoThing() *VideoThing {
	key := &pb.Key{}
	err := json.Unmarshal([]byte(`{"type":1,"private":"xIbkrrbgy24ps/HizaIsik1X0oAO2CSq9bAFDHa5QtfS4l/CTqSzU7BlqiQa1cOeQR94FZCN0RJuqoYgirV+Mg==","public":"0uJfwk6ks1OwZaokGtXDnkEfeBWQjdESbqqGIIq1fjI="}`), &key)
	if err != nil {
		panic(err)
	}

	return &VideoThing{
		key: key,
	}
}

type videoPublisher struct {
	// p vpn.PeerIndexPublisher
	s SwarmNetwork
}

// VideoThing ...
type VideoThing struct {
	p   []videoPublisher
	key *pb.Key
	s   *encoding.Swarm
	w   *chunkstream.Writer
}

// PublishSwarm ...
func (t *VideoThing) PublishSwarm(svc NetworkServices) error {
	// peers, err := getPeersGetter(svc, t.key, []byte("video"))()
	// if err != nil {
	// 	return err
	// }

	// for _, peer := range peers {
	// 	svc.PeerExchange.Connect(peer.HostID)
	// }

	// p, err := svc.PeerIndex.Publish(t.key.Public, []byte("video"), 0)
	// if err != nil {
	// 	return err
	// }

	// svc.Swarms.OpenSwarm(t.s)

	// t.p = append(t.p, videoPublisher{p, svc.Swarms})
	return nil
}

// Stop ...
func (t *VideoThing) Stop() {
	id := encoding.NewSwarmID(t.key.Public)
	for _, p := range t.p {
		// p.p.Stop()
		p.s.CloseSwarm(id)
	}
}

// RunClient ...
func (t *VideoThing) RunClient(ch chan *pb.VideoClientEvent) error {
	s, err := encoding.NewSwarm(
		encoding.NewSwarmID(t.key.Public),
		// encoding.NewDefaultSwarmOptions(),
		encoding.SwarmOptions{
			LiveWindow: 1 << 14, // 8MB
		},
	)
	if err != nil {
		return err
	}

	t.s = s

	// time.Sleep(2 * time.Second)
	r := t.s.Reader()
	cr, err := chunkstream.NewReaderSize(r, int64(r.Offset()), chunkstream.MaxSize)
	if err != nil {
		panic(err)
	}

	log.Println("offset", r.Offset())

	// TODO: hack - discard first fragment
	{
		var b bytes.Buffer
		if _, err := io.Copy(&b, cr); err != nil {
			panic(err)
		}
		b.Reset()
	}

	var seq int
	var bufs [32][32 * 1024]byte
	for {
		b := &bufs[seq%len(bufs)]
		seq++

		var n int
		var flush bool
		for {
			nn, err := cr.Read(b[n:])
			if err != nil && err != io.EOF {
				panic(err)
			}

			n += nn
			flush = err == io.EOF

			if n == len(b) || flush {
				break
			}
		}

		ch <- &pb.VideoClientEvent{
			Body: &pb.VideoClientEvent_Data_{
				Data: &pb.VideoClientEvent_Data{
					Data:  b[:n],
					Flush: flush,
				},
			},
		}
	}

	// var seq int
	// var bufs [3]bytes.Buffer
	// for {
	// 	b := &bufs[seq%len(bufs)]
	// 	b.Reset()
	// 	seq++

	// 	if _, err := io.Copy(b, cr); err != nil {
	// 		panic(err)
	// 	}

	// 	// log.Println("read bytes", b.Len(), spew.Sdump(b.Bytes()))

	// 	ch <- &pb.VideoClientEvent{
	// 		Body: &pb.VideoClientEvent_Data_{
	// 			Data: &pb.VideoClientEvent_Data{
	// 				Data:  b.Bytes(),
	// 				Flush: true,
	// 			},
	// 		},
	// 	}
	// }
}

// RunServer ...
func (t *VideoThing) RunServer() error {
	w, err := encoding.NewWriter(encoding.SwarmWriterOptions{
		// SwarmOptions: encoding.NewDefaultSwarmOptions(),
		SwarmOptions: encoding.SwarmOptions{
			LiveWindow: 1 << 14, // 8MB
		},
		Key: t.key,
	})
	if err != nil {
		log.Println("error creating writer", err)
		return err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.MaxSize)
	if err != nil {
		panic(err)
	}

	t.s = w.Swarm()
	t.w = cw
	return nil
}

// Write ...
func (t *VideoThing) Write(b []byte) error {
	if _, err := t.w.Write(b); err != nil {
		return err
	}
	return nil
}

// Flush ...
func (t *VideoThing) Flush() error {
	return t.w.Flush()
}
