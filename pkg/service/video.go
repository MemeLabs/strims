package service

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"log"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/encoding"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

func testKey() *pb.Key {
	key := &pb.Key{}
	err := json.Unmarshal([]byte(`{"type":1,"private":"xIbkrrbgy24ps/HizaIsik1X0oAO2CSq9bAFDHa5QtfS4l/CTqSzU7BlqiQa1cOeQR94FZCN0RJuqoYgirV+Mg==","public":"0uJfwk6ks1OwZaokGtXDnkEfeBWQjdESbqqGIIq1fjI="}`), &key)
	if err != nil {
		panic(err)
	}
	return key
}

var videoSalt = []byte("video")

// SwarmPublisher ...
type SwarmPublisher interface {
	PublishSwarm(svc *NetworkServices) error
}

// NewVideoServer ...
func NewVideoServer() (*VideoServer, error) {
	key := testKey()

	w, err := encoding.NewWriter(encoding.SwarmWriterOptions{
		// SwarmOptions: encoding.NewDefaultSwarmOptions(),
		SwarmOptions: encoding.SwarmOptions{
			LiveWindow: 1 << 15, // 32mb
		},
		Key: key,
	})
	if err != nil {
		log.Println("error creating writer", err)
		return nil, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.MaxSize)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	return &VideoServer{
		VideoSwarm: VideoSwarm{
			ctx:   ctx,
			close: cancel,
			key:   key.Public,
			s:     w.Swarm(),
		},
		w: cw,
	}, nil
}

// VideoServer ...
type VideoServer struct {
	VideoSwarm
	w *chunkstream.Writer
}

// Write ...
func (t *VideoServer) Write(b []byte) (int, error) {
	return t.w.Write(b)
}

// Flush ...
func (t *VideoServer) Flush() error {
	return t.w.Flush()
}

// NewVideoClient ...
func NewVideoClient() (*VideoClient, error) {
	key := testKey()

	s, err := encoding.NewSwarm(
		encoding.NewSwarmID(key.Public),
		// encoding.NewDefaultSwarmOptions(),
		encoding.SwarmOptions{
			LiveWindow: 1 << 15, // 32mb
		},
	)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	return &VideoClient{
		VideoSwarm: VideoSwarm{
			ctx:   ctx,
			close: cancel,
			key:   key.Public,
			s:     s,
		},
	}, nil
}

// VideoClient ...
type VideoClient struct {
	VideoSwarm
}

// SendEvents ...
func (c *VideoClient) SendEvents(ch chan *pb.VideoClientEvent) {
	r := c.s.Reader()
	log.Println("got swarm reader", r.Offset())
	cr, err := chunkstream.NewReaderSize(r, int64(r.Offset()), chunkstream.MaxSize)
	if err != nil {
		panic(err)
	}
	log.Println("opened chunkstream reader")

	// TODO: hack - discard first fragment
	{
		var b bytes.Buffer
		if _, err := io.Copy(&b, cr); err != nil {
			panic(err)
		}
		b.Reset()
	}

	log.Println("finished discarding chunk fragment")

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
}

// VideoSwarm ...
type VideoSwarm struct {
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	key       []byte
	s         *encoding.Swarm
	svc       []*NetworkServices
}

// PublishSwarm ...
func (t *VideoSwarm) PublishSwarm(svc *NetworkServices) error {
	svc.Swarms.OpenSwarm(t.s)

	newSwarmPeerManager(t.ctx, svc, getPeersGetter(t.ctx, svc, t.key, videoSalt))

	if err := svc.PeerIndex.Publish(t.ctx, t.key, videoSalt, 0); err != nil {
		return err
	}

	listing := &pb.DirectoryListing{
		MimeType: "video/webm",
		Title:    "test",
		Key:      t.key,
	}
	if err := svc.Directory.Publish(listing); err != nil {
		return err
	}

	t.svc = append(t.svc, svc)

	return nil
}

// Stop ...
func (t *VideoSwarm) Stop() {
	t.close()
	for _, svc := range t.svc {
		svc.Swarms.CloseSwarm(t.s.ID)
	}
}
