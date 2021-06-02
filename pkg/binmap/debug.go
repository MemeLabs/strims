package binmap

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

// DebugParseMap parse the debug format for building test from log output...
func DebugParseMap(s string) (m *Map) {
	m = &Map{}

	cellRx := regexp.MustCompile(`^\s*(\d+)\s+(\d+)\s(\w)(?:\s+(\d+)\s(\w))?$`)

	for i, l := range strings.Split(s, "\n") {
		if i == 0 {
			fmt.Sscanf(
				l,
				"freeTop: %d, allocCount: %d, cellCount: %d, rootBin: %d",
				&m.freeTop,
				&m.allocCount,
				&m.cellCount,
				&m.rootBin,
			)
		} else {
			match := cellRx.FindStringSubmatch(l)
			if len(match) == 0 {
				continue
			}

			var left, right uint64
			if match[3] == "F" || match[3] == "R" {
				left, _ = strconv.ParseUint(match[2], 10, 32)
			} else {
				left, _ = strconv.ParseUint(match[2], 2, 32)
			}
			if match[5] == "R" {
				right, _ = strconv.ParseUint(match[4], 10, 32)
			} else {
				right, _ = strconv.ParseUint(match[4], 2, 32)
			}
			m.cells = append(m.cells, cell{uint32(left), uint32(right)})
		}
	}
	return
}

func DebugPrintMap(m *Map) string {
	b := strings.Builder{}

	b.WriteString(fmt.Sprintf(
		"freeTop: %d, allocCount: %d, cellCount: %d, rootBin: %d\n",
		m.freeTop,
		m.allocCount,
		m.cellCount,
		m.rootBin,
	))

	f := freeCell{&m.cells[m.freeTop]}
	freeRefs := map[ref]bool{m.freeTop: true}
	for f.NextRef() != 0 && int(f.NextRef()) < len(m.cells) {
		freeRefs[f.NextRef()] = true
		f = freeCell{&m.cells[f.NextRef()]}
	}

	for i, c := range m.cells {
		b.WriteString(fmt.Sprintf("%-7d", i))

		if ref(i).IsMapRef() {
			b.WriteString(fmt.Sprintf(
				"%032s M %032s M",
				strconv.FormatUint(uint64(c.left), 2),
				strconv.FormatUint(uint64(c.right), 2),
			))
		} else if freeRefs[ref(i)] {
			b.WriteString(fmt.Sprintf("% 32s F", strconv.FormatUint(uint64(c.left), 10)))
		} else {
			r := ref(i)
			mc := m.mapCell(r)

			if mc.LeftRef() {
				b.WriteString(fmt.Sprintf("% 32s R", strconv.FormatUint(uint64(c.left), 10)))
			} else {
				b.WriteString(fmt.Sprintf("%032s B", strconv.FormatUint(uint64(c.left), 2)))
			}
			b.WriteString(" ")
			if mc.RightRef() {
				b.WriteString(fmt.Sprintf("% 32s R", strconv.FormatUint(uint64(c.right), 10)))
			} else {
				b.WriteString(fmt.Sprintf("%032s B", strconv.FormatUint(uint64(c.right), 2)))
			}
		}
		b.WriteString("\n")
	}

	return b.String()
}
