package frontend

import (
	"context"
	"io"

	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
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
	app control.AppControl
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
		defer r.Close()

		ch <- &videov1.EgressOpenStreamResponse{
			Body: &videov1.EgressOpenStreamResponse_Open_{
				Open: &videov1.EgressOpenStreamResponse_Open{
					TransferId: transferID,
				},
			},
		}

		var seq int
		var bufs [3][32 * 1024]byte
		for {
			b := &bufs[seq%len(bufs)]
			seq++

			var n int
			var segmentEnd, bufferUnderrun bool
		ReadLoop:
			for n < len(b) {
				nn, err := r.Read(b[n:])
				n += nn

				switch err {
				case nil:
				case io.EOF:
					segmentEnd = true
					break ReadLoop
				case store.ErrBufferUnderrun:
					bufferUnderrun = true
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

			ch <- &videov1.EgressOpenStreamResponse{
				Body: &videov1.EgressOpenStreamResponse_Data_{
					Data: &videov1.EgressOpenStreamResponse_Data{
						Data:           b[:n],
						SegmentEnd:     segmentEnd,
						BufferUnderrun: bufferUnderrun,
					},
				},
			}
		}
	}()
	return ch, nil
}
