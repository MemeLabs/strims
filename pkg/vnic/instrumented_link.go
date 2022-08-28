// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"reflect"

	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var (
	linkReadBytes = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_read_bytes",
		Help: "The total number of bytes read from network links",
	}, []string{"hostID", "type"})
	linkWriteBytes = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_write_bytes",
		Help: "The total number of bytes written to network links",
	}, []string{"hostID", "type"})
	linkReadCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_read_count",
		Help: "The total number of read ops from network links",
	}, []string{"hostID", "type"})
	linkWriteCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_link_write_count",
		Help: "The total number of write ops to network links",
	}, []string{"hostID", "type"})
)

func instrumentLink(l Link, hostID kademlia.ID) *instrumentedLink {
	metricLabels := instrumentedLinkMetricLabels(l, hostID)
	return &instrumentedLink{
		Link:       l,
		readBytes:  linkReadBytes.WithLabelValues(metricLabels...),
		writeBytes: linkWriteBytes.WithLabelValues(metricLabels...),
		readCount:  linkReadCount.WithLabelValues(metricLabels...),
		writeCount: linkWriteCount.WithLabelValues(metricLabels...),
	}
}

type instrumentedLink struct {
	Link
	readBytes  prometheus.Counter
	writeBytes prometheus.Counter
	readCount  prometheus.Counter
	writeCount prometheus.Counter
}

func (l *instrumentedLink) Read(p []byte) (int, error) {
	n, err := l.Link.Read(p)
	l.readBytes.Add(float64(n))
	l.readCount.Add(1)
	return n, err
}

func (l *instrumentedLink) Write(p []byte) (int, error) {
	n, err := l.Link.Write(p)
	l.writeBytes.Add(float64(n))
	l.writeCount.Add(1)
	return n, err
}

func deleteInstrumentedLinkMetrics(l Link, hostID kademlia.ID) {
	metricLabels := instrumentedLinkMetricLabels(l, hostID)
	linkReadBytes.DeleteLabelValues(metricLabels...)
	linkWriteBytes.DeleteLabelValues(metricLabels...)
	linkReadCount.DeleteLabelValues(metricLabels...)
	linkWriteCount.DeleteLabelValues(metricLabels...)
}

func instrumentedLinkMetricLabels(l Link, hostID kademlia.ID) []string {
	if il, ok := l.(*instrumentedLink); ok {
		l = il.Link
	}
	return []string{hostID.String(), reflect.TypeOf(l).String()}
}
