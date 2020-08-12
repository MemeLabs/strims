package main

import (
	"encoding/csv"
	"errors"
	"flag"
	"fmt"
	"net/http"
	"strconv"
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.uber.org/zap"
)

var opt = exporterOptions{
	Regions: []string{"sfo", "tor", "nyc", "lon", "ams", "fra", "blr", "sgp"},
}

func init() {
	flag.StringVar(&opt.Namespace, "namespace", "", "prometheus namespace")
	flag.StringVar(&opt.Username, "username", "", "haproxy username")
	flag.StringVar(&opt.Password, "password", "", "haproxy password")
	flag.StringVar(&opt.Domain, "domain", "", "haproxy domain base")
	flag.UintVar(&opt.StatsPort, "port", 8080, "haproxy stats port")
	flag.IntVar(&opt.MaxRegionSize, "max-region-size", 50, "max hosts per region")
	flag.StringVar(&opt.ServerName, "server-name", "external", "haproxy server name")
}

func main() {
	flag.Parse()

	logger, err := zap.NewProduction()
	if err != nil {
		panic(err)
	}

	e := newExporter(logger, opt)
	go e.ScrapeSizes()

	prometheus.MustRegister(e)
	http.Handle("/metrics", promhttp.Handler())
	http.ListenAndServe(":2112", nil)
}

type exporterOptions struct {
	Regions       []string
	Namespace     string
	Username      string
	Password      string
	Domain        string
	StatsPort     uint
	MaxRegionSize int
	ServerName    string
}

func newExporter(logger *zap.Logger, opt exporterOptions) *exporter {
	return &exporter{
		logger:        logger,
		regionSizes:   map[string]int{},
		regions:       opt.Regions,
		urlFormat:     fmt.Sprintf("https://%s:%s@%%s%%d.%s:%d/stats;csv;norefresh", opt.Username, opt.Password, opt.Domain, opt.StatsPort),
		maxRegionSize: opt.MaxRegionSize,
		serverName:    opt.ServerName,

		totalScrapes: prometheus.NewCounter(prometheus.CounterOpts{
			Namespace: opt.Namespace,
			Name:      "total_scrapes",
		}),
		totalErrors: prometheus.NewCounter(prometheus.CounterOpts{
			Namespace: opt.Namespace,
			Name:      "total_errors",
		}),
		sessionsTotal: newMetric(opt.Namespace, "sessions_total", "Total number of sessions.", prometheus.CounterValue, nil),
		bytesInTotal:  newMetric(opt.Namespace, "bytes_in_total", "Current total of incoming bytes.", prometheus.CounterValue, nil),
		bytesOutTotal: newMetric(opt.Namespace, "bytes_out_total", "Current total of outgoing bytes.", prometheus.CounterValue, nil),
	}
}

type exporter struct {
	logger *zap.Logger

	collectMu     sync.Mutex
	regionSizesMu sync.Mutex
	regionSizes   map[string]int

	regions       []string
	urlFormat     string
	maxRegionSize int
	serverName    string

	totalScrapes  prometheus.Counter
	totalErrors   prometheus.Counter
	sessionsTotal metricInfo
	bytesInTotal  metricInfo
	bytesOutTotal metricInfo
}

var metricLabels = []string{"region", "index"}

func newMetric(namespace string, metricName string, docString string, t prometheus.ValueType, constLabels prometheus.Labels) metricInfo {
	return metricInfo{
		Desc: prometheus.NewDesc(
			prometheus.BuildFQName(namespace, "frontend", metricName),
			docString,
			metricLabels,
			constLabels,
		),
		Type: t,
	}
}

type metricInfo struct {
	Desc *prometheus.Desc
	Type prometheus.ValueType
}

func (e *exporter) ScrapeSizes() {
	t := time.NewTicker(time.Minute)
	for {
		for _, r := range e.regions {
			go e.scrapeRegionSize(r)
		}
		<-t.C
	}
}

func (e *exporter) formatURL(region string, index int) string {
	return fmt.Sprintf(e.urlFormat, region, index)
}

func (e *exporter) scrapeRegionSize(region string) {
	var size int
	for i := 1; i <= e.maxRegionSize; i++ {
		size = i
		res, err := http.Head(e.formatURL(region, i))
		if err != nil || res.StatusCode != http.StatusOK {
			break
		}
	}

	e.regionSizesMu.Lock()
	defer e.regionSizesMu.Unlock()
	e.regionSizes[region] = size
}

// Describe implements prometheus.Collector.
func (e *exporter) Describe(ch chan<- *prometheus.Desc) {
	ch <- e.totalScrapes.Desc()
	ch <- e.totalErrors.Desc()
	ch <- e.bytesInTotal.Desc
	ch <- e.bytesOutTotal.Desc
	ch <- e.sessionsTotal.Desc
}

// Collect implements prometheus.Collector.
func (e *exporter) Collect(ch chan<- prometheus.Metric) {
	e.collectMu.Lock()
	defer e.collectMu.Unlock()

	e.logger.Info("began collecting metrics")
	start := time.Now()

	var wg sync.WaitGroup
	e.regionSizesMu.Lock()
	for region, size := range e.regionSizes {
		for i := 1; i < size; i++ {
			wg.Add(1)
			go func(region string, index int) {
				e.totalScrapes.Inc()
				if err := e.scrape(ch, region, index); err != nil {
					e.logger.Error(
						"scrape failed",
						zap.String("region", region),
						zap.Int("index", index),
						zap.Error(err),
					)
					e.totalErrors.Inc()
				}
				wg.Done()
			}(region, i)
		}
	}
	e.regionSizesMu.Unlock()
	wg.Wait()

	e.logger.Info("finished collecting metrics", zap.Duration("duration", time.Since(start)))

	ch <- e.totalScrapes
	ch <- e.totalErrors
}

func (e *exporter) scrape(ch chan<- prometheus.Metric, region string, index int) error {
	res, err := http.Get(e.formatURL(region, index))
	if err != nil {
		return err
	}

	records, err := csv.NewReader(res.Body).ReadAll()
	if err != nil {
		return err
	}

	labels := []string{region, strconv.Itoa(index)}

	for i := 1; i < len(records); i++ {
		if len(records[i]) < 1 {
			continue
		}
		if records[i][0] == e.serverName {
			sessionsTotal, err := strconv.ParseFloat(records[i][7], 64)
			if err != nil {
				return err
			}
			bytesInTotal, err := strconv.ParseFloat(records[i][8], 64)
			if err != nil {
				return err
			}
			bytesOutTotal, err := strconv.ParseFloat(records[i][9], 64)
			if err != nil {
				return err
			}

			ch <- prometheus.MustNewConstMetric(e.sessionsTotal.Desc, e.sessionsTotal.Type, sessionsTotal, labels...)
			ch <- prometheus.MustNewConstMetric(e.bytesInTotal.Desc, e.bytesInTotal.Type, bytesInTotal, labels...)
			ch <- prometheus.MustNewConstMetric(e.bytesOutTotal.Desc, e.bytesOutTotal.Type, bytesOutTotal, labels...)

			return nil
		}
	}

	return errors.New("server not found")
}
