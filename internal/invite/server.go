// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package invite

import (
	"context"
	"net/http"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1invite "github.com/MemeLabs/strims/pkg/apis/network/v1/invite"
	"github.com/MemeLabs/strims/pkg/kv"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

func NewServer(logger *zap.Logger, store kv.BlobStore) (http.Handler, error) {
	if err := store.CreateStoreIfNotExists("invites"); err != nil {
		return nil, err
	}

	h := rpc.NewHTTPServer(logger)
	networkv1invite.RegisterInviteLinkService(h, &inviteService{
		logger: logger,
		store:  store,
	})
	return h, nil
}

type Server struct {
	*rpc.HTTPServer
}

type inviteService struct {
	networkv1invite.UnimplementedInviteLinkService
	logger *zap.Logger
	store  kv.BlobStore
}

func (s *inviteService) GetInvitation(ctx context.Context, req *networkv1invite.GetInvitationRequest) (*networkv1invite.GetInvitationResponse, error) {
	invitation := &networkv1.Invitation{}
	err := s.store.View("invites", func(tx kv.BlobTx) error {
		b, err := tx.Get(req.Code)
		if err != nil {
			return err
		}
		return proto.Unmarshal(b, invitation)
	})
	if err != nil {
		return nil, err
	}
	return &networkv1invite.GetInvitationResponse{Invitation: invitation}, nil
}
