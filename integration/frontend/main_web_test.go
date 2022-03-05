//go:build js

package frontend

import (
	"github.com/MemeLabs/go-ppspp/integration/driver"
)

var NewDriver = driver.NewWeb
