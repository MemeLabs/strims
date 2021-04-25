package debug

import (
	"log"
	"sort"
	"strconv"
	"strings"
	"sync/atomic"
	"time"
)

var counts = [128]int64{}

func IncrCounter(i int) {
	atomic.AddInt64(&counts[i], 1)
}

func init() {
	go func() {
		indices := [128]int{}
		current := [128]int64{}

		t := time.NewTicker(time.Second)
		for range t.C {
			for i := 0; i < len(counts); i++ {
				indices[i] = i
				current[i] = atomic.SwapInt64(&counts[i], 0)
			}

			s := (&counterFormatter{indices, current}).String()
			if s != "" {
				log.Println(s)
			}
		}
	}()
}

type counterFormatter struct {
	indices [128]int
	current [128]int64
}

func (m *counterFormatter) Len() int           { return len(m.indices) }
func (m *counterFormatter) Less(i, j int) bool { return m.current[i] > m.current[j] }
func (m *counterFormatter) Swap(i, j int) {
	m.indices[i], m.indices[j] = m.indices[j], m.indices[i]
	m.current[i], m.current[j] = m.current[j], m.current[i]
}

func (m *counterFormatter) String() string {
	sort.Sort(m)
	var b strings.Builder
	for i := 0; i < len(m.indices); i++ {
		if m.current[i] == 0 {
			break
		}
		if i != 0 {
			b.WriteString(", ")
		}
		b.WriteString(strconv.FormatInt(int64(m.indices[i]), 10))
		b.WriteString(": ")
		b.WriteString(strconv.FormatInt(m.current[i], 10))
	}
	return b.String()
}
