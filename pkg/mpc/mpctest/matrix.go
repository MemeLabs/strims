// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpctest

import (
	"fmt"
	"log"
	"strings"
)

// PrintMatrix ...
func PrintMatrix(m []byte, rows, cols, width int) {
	var s strings.Builder
	s.WriteString(fmt.Sprintf("%d x %d (%d)", rows, cols, cols*8))
	for i := 0; i < len(m); i++ {
		if i%cols == 0 {
			s.WriteRune('\n')
		}
		if i%(cols*width) == 0 {
			s.WriteRune('\n')
		}
		g := fmt.Sprintf("%08b ", m[i])
		gs := []string{}
		if width > 8 {
			gs = append(gs, g)
		}
		for j := width; j <= 8; j += width {
			gs = append(gs, g[j-width:j])
		}
		s.WriteString(strings.Join(gs, " "))
		s.WriteRune(' ')
	}
	log.Println(s.String())
}
