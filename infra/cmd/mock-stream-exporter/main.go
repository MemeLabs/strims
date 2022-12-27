package main

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/bytereader"
	"github.com/golang/protobuf/proto"
	dto "github.com/prometheus/client_model/go"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
)

func main() {
	var s scraper
	flag.StringVar(&s.Namespace, "namespace", "strims", "pod namespace")
	flag.StringVar(&s.PodLabelSelector, "label", "strims.gg/svc=leader", "pod label selector")
	flag.StringVar(&s.MetricName, "metric", "strims_debug_mock_stream_latency_ms", "prometheus metric name")
	addr := flag.String("addr", ":8080", "http server address")
	flag.Parse()

	var mux http.ServeMux
	mux.HandleFunc("/metrics", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		m, err := s.scrape(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		encoder := json.NewEncoder(w)
		encoder.SetIndent("", "\t")
		encoder.Encode(m)
	})
	fmt.Println(http.ListenAndServe(*addr, &mux))
}

type NodeMetrics struct {
	NodeName string
	Metrics  []*dto.Metric
}

type scraper struct {
	Namespace        string
	PodLabelSelector string
	MetricName       string
}

func (s *scraper) scrape(ctx context.Context) ([]*NodeMetrics, error) {
	config, err := rest.InClusterConfig()
	if err != nil {
		return nil, err
	}

	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		return nil, err
	}

	pods, err := clientset.CoreV1().Pods(s.Namespace).List(context.TODO(), metav1.ListOptions{
		LabelSelector: s.PodLabelSelector,
	})
	if err != nil {
		return nil, err
	}

	var resLock sync.Mutex
	var res []*NodeMetrics

	var wg sync.WaitGroup
	wg.Add(len(pods.Items))
	for _, pod := range pods.Items {
		pod := pod
		go func() {
			defer wg.Done()

			m, err := s.scrapeNode(ctx, pod.Status.PodIP)
			if err != nil {
				return
			}

			resLock.Lock()
			res = append(res, &NodeMetrics{
				NodeName: pod.Spec.NodeName,
				Metrics:  m,
			})
			resLock.Unlock()
		}()
	}
	wg.Wait()

	return res, nil
}

func (s *scraper) scrapeNode(ctx context.Context, addr string) ([]*dto.Metric, error) {
	req, err := http.NewRequest(http.MethodGet, fmt.Sprintf("http://%s:1971/metrics", addr), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Add("Accept", "application/vnd.google.protobuf; proto=io.prometheus.client.MetricFamily;encoding=delimited")
	res, err := http.DefaultClient.Do(req.WithContext(ctx))
	if err != nil {
		return nil, err
	}

	var buf bytes.Buffer
	for {
		l, err := binary.ReadUvarint(bytereader.New(res.Body))
		if err != nil {
			return nil, err
		}

		buf.Reset()
		if _, err := io.CopyN(&buf, res.Body, int64(l)); err != nil {
			return nil, err
		}

		var m dto.MetricFamily
		if err := proto.Unmarshal(buf.Bytes(), &m); err != nil {
			return nil, err
		}

		if m.GetName() == s.MetricName {
			return m.Metric, nil
		}
	}
}
