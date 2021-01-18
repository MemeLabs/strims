package frontend

import (
	"context"

	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		videov1.RegisterCaptureService(server, &videoCaptureService{
			app: params.App,
		})
	})
}

// videoCaptureService ...
type videoCaptureService struct {
	app control.AppControl
}

// Open ...
func (s *videoCaptureService) Open(ctx context.Context, r *videov1.CaptureOpenRequest) (*videov1.CaptureOpenResponse, error) {
	id, err := s.app.VideoCapture().Open(r.MimeType, r.DirectorySnippet, r.NetworkKeys)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureOpenResponse{Id: id}, nil
}

// Update ...
func (s *videoCaptureService) Update(ctx context.Context, r *videov1.CaptureUpdateRequest) (*videov1.CaptureUpdateResponse, error) {
	err := s.app.VideoCapture().Update(r.Id, r.DirectorySnippet)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureUpdateResponse{}, err
}

// Append ...
func (s *videoCaptureService) Append(ctx context.Context, r *videov1.CaptureAppendRequest) (*videov1.CaptureAppendResponse, error) {
	err := s.app.VideoCapture().Append(r.Id, r.Data, r.SegmentEnd)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureAppendResponse{}, err
}

// Close ...
func (s *videoCaptureService) Close(ctx context.Context, r *videov1.CaptureCloseRequest) (*videov1.CaptureCloseResponse, error) {
	err := s.app.VideoCapture().Close(r.Id)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureCloseResponse{}, err
}
