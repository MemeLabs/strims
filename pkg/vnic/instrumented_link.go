package vnic

import (
	"sync/atomic"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var (
	linkReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_link_read_bytes",
		Help: "The total number of bytes read from network links",
	})
	linkWriteBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_link_write_bytes",
		Help: "The total number of bytes written to network links",
	})
)

type InstrumentedLink interface {
	ReadMetrics() LinkMetrics
}

type LinkMetrics struct {
	OpenedAt   timeutil.Time
	WriteCount uint64
	ReadCount  uint64
	WriteBytes uint64
	ReadBytes  uint64
}

func instrumentLink(l Link) *instrumentedLink {
	return &instrumentedLink{
		Link: l,
		LinkMetrics: LinkMetrics{
			OpenedAt: timeutil.Now(),
		},
	}
}

type instrumentedLink struct {
	Link
	label atomic.Value
	LinkMetrics
}

func (l *instrumentedLink) ReadMetrics() LinkMetrics {
	return LinkMetrics{
		l.OpenedAt,
		atomic.LoadUint64(&l.WriteCount),
		atomic.LoadUint64(&l.ReadCount),
		atomic.LoadUint64(&l.WriteBytes),
		atomic.LoadUint64(&l.ReadBytes),
	}
}

func (l *instrumentedLink) Read(p []byte) (int, error) {
	n, err := l.Link.Read(p)
	linkReadBytes.Add(float64(n))
	atomic.AddUint64(&l.WriteCount, 1)
	atomic.AddUint64(&l.WriteBytes, uint64(n))
	return n, err
}

func (l *instrumentedLink) Write(p []byte) (int, error) {
	n, err := l.Link.Write(p)
	linkWriteBytes.Add(float64(n))
	atomic.AddUint64(&l.ReadCount, 1)
	atomic.AddUint64(&l.ReadBytes, uint64(n))
	return n, err
}
