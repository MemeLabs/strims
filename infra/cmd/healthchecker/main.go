package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"os"
	"os/signal"
	"sync"
	"time"

	"github.com/cloudflare/cloudflare-go"
	"github.com/hashicorp/raft"
	"gopkg.in/yaml.v3"
)

var (
	address    = flag.String("address", "", "TCP host+port for this node")
	configPath = flag.String("config", "config.yaml", "Config file path")
)

type config struct {
	HealthCheckers []string `yaml:"healthCheckers"`
	Cloudflare     struct {
		Token  string `yaml:"token"`
		ZoneID string `yaml:"zoneID"`
		Domain string `yaml:"domain"`
	} `yaml:"cloudflare"`
	LoadBalancers []string      `yaml:"loadBalancers"`
	CheckInterval time.Duration `yaml:"checkInterval"`
}

func main() {
	flag.Parse()

	if *address == "" {
		log.Fatal("no value provided for address argument")
	}

	f, err := os.Open(*configPath)
	if err != nil {
		log.Fatalf("failed to open config: %v", err)
	}

	var c config
	yaml.NewDecoder(f).Decode(&c)

	var urls []*url.URL
	for _, ru := range c.LoadBalancers {
		u, err := url.Parse(ru)
		if err != nil {
			log.Fatalf("error parsing load balancer URL: %v", err)
		}
		urls = append(urls, u)
	}

	d, err := NewDNSAPI(c.Cloudflare.Token, c.Cloudflare.ZoneID, c.Cloudflare.Domain)
	if err != nil {
		log.Fatalf("failed to create dns client: %v", err)
	}

	t := time.NewTicker(c.CheckInterval)
	var leader bool

	r, err := NewRaft(*address, c.HealthCheckers)
	if err != nil {
		log.Fatalf("failed to start raft: %v", err)
	}

	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cancel()

	for {
		select {
		case <-t.C:
			if leader {
				if err := Run(ctx, d, urls); err != nil {
					log.Printf("Run: %v", err)
				}
			}
		case leader = <-r.LeaderCh():
		case <-ctx.Done():
			return
		}
	}
}

// Run updates the DNS record with the hostname of the first URL to respond
// successfully if the URL with the record's current value failed.
func Run(ctx context.Context, d *DNSAPI, urls []*url.URL) error {
	healthyURLs := checkURLs(urls)
	if len(healthyURLs) == 0 {
		return errors.New("checkURLS: all healthchecks failed")
	}

	r, err := d.GetRecord(ctx)
	if err != nil {
		return fmt.Errorf("DNSAPI.GetRecord: %w", err)
	}

	for _, u := range healthyURLs {
		if u.Hostname() == r.Content {
			return nil
		}
	}

	r.Content = healthyURLs[0].Hostname()
	if err := d.UpdateRecord(ctx, r); err != nil {
		return fmt.Errorf("DNSAPI.UpdateRecord: %w", err)
	}
	return nil
}

// checkURLs returns the subset of the supplied URLs that respond with HTTP
// status 200.
func checkURLs(urls []*url.URL) []*url.URL {
	var mu sync.Mutex
	var healthy []*url.URL

	var wg sync.WaitGroup
	wg.Add(len(urls))

	for _, u := range urls {
		u := u
		go func() {
			res, err := http.Get(u.String())
			if err == nil && res.StatusCode == http.StatusOK {
				mu.Lock()
				healthy = append(healthy, u)
				mu.Unlock()
			}
			wg.Done()
		}()
	}

	wg.Wait()

	return healthy
}

func NewDNSAPI(token, zoneID, domain string) (*DNSAPI, error) {
	api, err := cloudflare.NewWithAPIToken(token)
	if err != nil {
		return nil, err
	}
	return &DNSAPI{
		zoneID: zoneID,
		domain: domain,
		api:    api,
	}, nil
}

// DNSAPI wraps the Cloudflare API with helpers for getting/setting a single
// DNS record.
type DNSAPI struct {
	zoneID string
	domain string
	api    *cloudflare.API
}

func (d *DNSAPI) GetRecord(ctx context.Context) (cloudflare.DNSRecord, error) {
	records, err := d.api.DNSRecords(ctx, d.zoneID, cloudflare.DNSRecord{Name: d.domain})
	if err != nil {
		return cloudflare.DNSRecord{}, err
	}
	if len(records) == 0 {
		return cloudflare.DNSRecord{}, errors.New("record not found")
	}
	return records[0], nil
}

func (d *DNSAPI) UpdateRecord(ctx context.Context, r cloudflare.DNSRecord) error {
	return d.api.UpdateDNSRecord(ctx, d.zoneID, r.ID, r)
}

// NewRaft creates a minimal raft cluster to leverage the leader election
// mechanism...
func NewRaft(address string, peers []string) (*raft.Raft, error) {
	c := raft.DefaultConfig()
	c.LocalID = raft.ServerID(address)

	ldb := raft.NewInmemStore()
	sdb := raft.NewInmemStore()
	fss := raft.NewInmemSnapshotStore()

	tm, err := raft.NewTCPTransport(address, nil, 2, time.Second, os.Stderr)
	if err != nil {
		return nil, fmt.Errorf("raft.NewTCPTransport: %w", err)
	}

	r, err := raft.NewRaft(c, nil, ldb, sdb, fss, tm)
	if err != nil {
		return nil, fmt.Errorf("raft.NewRaft: %w", err)
	}

	servers := []raft.Server{
		{
			Suffrage: raft.Voter,
			ID:       raft.ServerID(address),
			Address:  raft.ServerAddress(address),
		},
	}

	for _, peer := range peers {
		if peer != address {
			servers = append(servers, raft.Server{
				Suffrage: raft.Voter,
				ID:       raft.ServerID(peer),
				Address:  raft.ServerAddress(peer),
			})
		}
	}

	f := r.BootstrapCluster(raft.Configuration{Servers: servers})
	if err := f.Error(); err != nil {
		return nil, fmt.Errorf("raft.Raft.BootstrapCluster: %w", err)
	}

	return r, nil
}
