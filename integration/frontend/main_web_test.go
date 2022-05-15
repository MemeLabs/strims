// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package frontend

import (
	"github.com/MemeLabs/strims/integration/driver"
)

var NewDriver = driver.NewWeb
