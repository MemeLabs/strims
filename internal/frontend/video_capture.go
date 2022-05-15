// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/transfer"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterCaptureService(server, &videoCaptureService{
			app: params.App,
		})
	})
}

// videoCaptureService ...
type videoCaptureService struct {
	app app.Control
}

// Open ...
func (s *videoCaptureService) Open(ctx context.Context, r *videov1.CaptureOpenRequest) (*videov1.CaptureOpenResponse, error) {
	id, err := s.app.VideoCapture().Open(r.MimeType, r.DirectorySnippet, r.NetworkKeys)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureOpenResponse{Id: id[:]}, nil
}

// Update ...
func (s *videoCaptureService) Update(ctx context.Context, r *videov1.CaptureUpdateRequest) (*videov1.CaptureUpdateResponse, error) {
	id, err := transfer.ParseID(r.Id)
	if err != nil {
		return nil, err
	}

	err = s.app.VideoCapture().Update(id, r.DirectorySnippet)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureUpdateResponse{}, err
}

// Append ...
func (s *videoCaptureService) Append(ctx context.Context, r *videov1.CaptureAppendRequest) (*videov1.CaptureAppendResponse, error) {
	id, err := transfer.ParseID(r.Id)
	if err != nil {
		return nil, err
	}

	err = s.app.VideoCapture().Append(id, r.Data, r.SegmentEnd)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureAppendResponse{}, err
}

// Close ...
func (s *videoCaptureService) Close(ctx context.Context, r *videov1.CaptureCloseRequest) (*videov1.CaptureCloseResponse, error) {
	id, err := transfer.ParseID(r.Id)
	if err != nil {
		return nil, err
	}

	err = s.app.VideoCapture().Close(id)
	if err != nil {
		return nil, err
	}
	return &videov1.CaptureCloseResponse{}, err
}
