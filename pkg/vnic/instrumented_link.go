// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var (
	linkReadBytes = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_read_bytes",
		Help: "The total number of bytes read from network links",
	}, []string{"hostID"})
	linkWriteBytes = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_write_bytes",
		Help: "The total number of bytes written to network links",
	}, []string{"hostID"})
)

func instrumentLink(l Link, hostID kademlia.ID) *instrumentedLink {
	return &instrumentedLink{
		Link:       l,
		readBytes:  linkReadBytes.WithLabelValues(hostID.String()),
		writeBytes: linkWriteBytes.WithLabelValues(hostID.String()),
	}
}

type instrumentedLink struct {
	Link
	readBytes  prometheus.Counter
	writeBytes prometheus.Counter
}

func (l *instrumentedLink) Read(p []byte) (int, error) {
	n, err := l.Link.Read(p)
	l.readBytes.Add(float64(n))
	return n, err
}

func (l *instrumentedLink) Write(p []byte) (int, error) {
	n, err := l.Link.Write(p)
	l.writeBytes.Add(float64(n))
	return n, err
}

func deleteInstrumentedLinkMetrics(hostID kademlia.ID) {
	linkReadBytes.DeleteLabelValues(hostID.String())
	linkWriteBytes.DeleteLabelValues(hostID.String())
}
