package frontend

import (
	"context"
	"io"

	"github.com/MemeLabs/go-ppspp/internal/app"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		videov1.RegisterEgressService(server, &videoEgressService{
			app: params.App,
		})
	})
}

// videoEgressService ...
type videoEgressService struct {
	app app.Control
}

// OpenStream ...
func (s *videoEgressService) OpenStream(ctx context.Context, r *videov1.EgressOpenStreamRequest) (<-chan *videov1.EgressOpenStreamResponse, error) {
	ch := make(chan *videov1.EgressOpenStreamResponse)
	go func() {
		defer close(ch)

		transferID, r, err := s.app.VideoEgress().OpenStream(r.SwarmUri, r.NetworkKeys)
		if err != nil {
			ch <- &videov1.EgressOpenStreamResponse{
				Body: &videov1.EgressOpenStreamResponse_Error_{
					Error: &videov1.EgressOpenStreamResponse_Error{
						Message: err.Error(),
					},
				},
			}
			return
		}

		go func() {
			<-ctx.Done()
			r.Close()
		}()

		ch <- &videov1.EgressOpenStreamResponse{
			Body: &videov1.EgressOpenStreamResponse_Open_{
				Open: &videov1.EgressOpenStreamResponse_Open{
					TransferId: transferID,
				},
			},
		}

		var i int
		var resPool [3]*videov1.EgressOpenStreamResponse
		for i := range resPool {
			resPool[i] = &videov1.EgressOpenStreamResponse{
				Body: &videov1.EgressOpenStreamResponse_Data_{
					Data: &videov1.EgressOpenStreamResponse_Data{
						Data: make([]byte, 32*1024),
					},
				},
			}
		}
		for {
			if i++; i == len(resPool) {
				i = 0
			}
			res := resPool[i]

			d := res.GetData()
			d.SegmentEnd = false
			d.BufferUnderrun = false
			d.Data = d.Data[:cap(d.Data)]

			var n int
		ReadLoop:
			for n < len(d.Data) {
				nn, err := r.Read(d.Data[n:])
				n += nn

				switch err {
				case nil:
				case io.EOF:
					d.SegmentEnd = true
					break ReadLoop
				case store.ErrBufferUnderrun:
					d.BufferUnderrun = true
					n = 0
					break ReadLoop
				default:
					ch <- &videov1.EgressOpenStreamResponse{
						Body: &videov1.EgressOpenStreamResponse_Error_{
							Error: &videov1.EgressOpenStreamResponse_Error{
								Message: err.Error(),
							},
						},
					}
					return
				}
			}

			d.Data = d.Data[:n]
			ch <- res
		}
	}()
	return ch, nil
}
