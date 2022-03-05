//go:build !web
// +build !web

package frontend

import (
	"github.com/MemeLabs/go-ppspp/integration/driver"
)

var NewDriver = driver.NewNative
