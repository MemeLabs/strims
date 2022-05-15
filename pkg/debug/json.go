// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"encoding/json"
	"fmt"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/strims/pkg/errutil"
)

// PrintJSON ...
func PrintJSON(i any) {
	_, file, line, _ := runtime.Caller(1)
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(errutil.Must(json.MarshalIndent(i, "", "  "))),
	)
}
