package service

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/json"
	"io"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/hls"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"go.uber.org/zap"
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
func NewVideoServer(logger *zap.Logger) (*VideoServer, error) {
	key := testKey()

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		// SwarmOptions: ppspp.NewDefaultSwarmOptions(),
		SwarmOptions: ppspp.SwarmOptions{
			LiveWindow: 1 << 15, // 32mb
			// LiveWindow: 1 << 16, // 64mb
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
			logger: logger,
			ctx:    ctx,
			close:  cancel,
			key:    key.Public,
			s:      w.Swarm(),
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
func NewVideoClient(logger *zap.Logger) (*VideoClient, error) {
	key := testKey()

	s, err := ppspp.NewSwarm(
		ppspp.NewSwarmID(key.Public),
		// ppspp.NewDefaultSwarmOptions(),
		ppspp.SwarmOptions{
			LiveWindow: 1 << 15, // 32mb
			// LiveWindow: 1 << 16, // 64mb
		},
	)
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	return &VideoClient{
		VideoSwarm: VideoSwarm{
			logger: logger,
			ctx:    ctx,
			close:  cancel,
			key:    key.Public,
			s:      s,
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
	var bufs [128][32 * 1024]byte
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

// SendStream ...
func (c *VideoClient) SendStream(ctx context.Context, stream *hls.Stream) error {
	r := c.s.Reader()
	log.Println("got swarm reader", r.Offset())
	cr, err := chunkstream.NewReaderSize(r, int64(r.Offset()), chunkstream.MaxSize)
	if err != nil {
		return err
	}
	log.Println("opened chunkstream reader")

	// TODO: hack - discard first fragment
	{
		var b bytes.Buffer
		if _, err := io.Copy(&b, cr); err != nil {
			return err
		}
		b.Reset()
	}

	log.Println("finished discarding chunk fragment")

	var headerRead, headerWritten bool
	var b [32 * 1024]byte
	w := stream.NextWriter()
	for {
		var n int
		var flush bool
		for {
			nn, err := cr.Read(b[n:])
			if err != nil && err != io.EOF {
				return err
			}

			n += nn
			flush = err == io.EOF

			if n == len(b) || flush {
				break
			}
		}

		p := b[:n]
		if !headerRead {
			headerLen := binary.BigEndian.Uint16(p)
			if !headerWritten {
				iw := stream.InitWriter()
				if _, err := iw.Write(p[2 : headerLen+2]); err != nil {
					return err
				}
				if err := iw.Close(); err != nil {
					return err
				}
				headerWritten = true
			}
			p = p[headerLen+2:]
			headerRead = true
		}

		if _, err := w.Write(p); err != nil {
			return err
		}

		if flush {
			if err := w.Close(); err != nil {
				return err
			}
			w = stream.NextWriter()
			headerRead = false
		}

		if err := ctx.Err(); err != nil {
			return err
		}
	}
}

// VideoSwarm ...
type VideoSwarm struct {
	logger    *zap.Logger
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	key       []byte
	s         *ppspp.Swarm
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
	if err := svc.Directory.Publish(t.ctx, listing); err != nil {
		return err
	}
	t.logger.Info("published video swarm", logutil.ByteHex("key", t.key))

	t.svc = append(t.svc, svc)

	return nil
}

func (t *VideoSwarm) unpublishSwarm(svc *NetworkServices) {
	svc.Swarms.CloseSwarm(t.s.ID())

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := svc.Directory.Unpublish(ctx, t.key); err != nil {
		t.logger.Info("failed to unpublish swarm", zap.Error(err))
	}
}

// Stop ...
func (t *VideoSwarm) Stop() {
	t.close()
	for _, svc := range t.svc {
		go t.unpublishSwarm(svc)
	}
}
