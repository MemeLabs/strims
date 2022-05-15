// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !web
// +build !web

package frontend

import (
	"github.com/MemeLabs/strims/integration/driver"
)

var NewDriver = driver.NewNative
