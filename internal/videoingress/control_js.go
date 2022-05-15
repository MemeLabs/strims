// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package videoingress

import (
	"context"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/transfer"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	vpn *vpn.Host,
	store *dao.ProfileStore,
	observers *event.Observers,
	profile *profilev1.Profile,
	transfer transfer.Control,
	network network.Control,
	directory directory.Control,
) Control {
	return &control{}
}

// Control ...
type control struct{}

// Run ...
func (c *control) Run() {}
