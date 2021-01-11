package pgsutil

import (
	"fmt"
	"strings"
)

// Generator ...
type Generator struct {
	indent int
	strings.Builder
}

// Linef ...
func (g *Generator) Linef(v string, args ...interface{}) {
	g.Line(fmt.Sprintf(v, args...))
}

// Line ...
func (g *Generator) Line(v string) {
	d := strings.Count(v, "{") - strings.Count(v, "}")
	if d < 0 {
		g.indent += d
	}
	g.WriteString(strings.Repeat("  ", g.indent))
	if d > 0 {
		g.indent += d
	}

	g.WriteString(v)
	g.LineBreak()
}

// LineBreak ...
func (g *Generator) LineBreak() {
	g.WriteRune('\n')
}
