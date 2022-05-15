// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"fmt"
	"runtime"
)

// Tracer stores the last location where Update was called
type Tracer struct {
	file string
	line int
}

func (l *Tracer) String() string {
	return fmt.Sprintf("%s:%d", l.file, l.line)
}

// Update last executed line
func (l *Tracer) Update(skip ...int) {
	n := 1
	if len(skip) != 0 {
		n = skip[0]
	}
	_, file, line, ok := runtime.Caller(n)
	if ok {
		l.file = file
		l.line = line
	}
}
