// frontend_driver is a chromedp driver running the frontend for testing
// https://github.com/MemeLabs/url-extract
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"strings"
	"time"

	"github.com/chromedp/cdproto/network"
	"github.com/chromedp/chromedp"
	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/api/types/strslice"
	"github.com/docker/docker/client"
	"github.com/docker/go-connections/nat"
)

var (
	target       string
	headlessURL  string
	heuristics   bool
	timeout      int
	quiet        bool
	setup        bool
	containerIDS []string
	cli          *client.Client
)

func init() {
	flag.StringVar(&target, "url", "https://strims.gg/", "the URL to analyze")
	flag.StringVar(&headlessURL, "remote", "localhost:9222", "the endpoint of the headless instance")
	flag.IntVar(&timeout, "timeout", 500, "time in seconds to wait for the site to load and a result to be detected")
	flag.BoolVar(&quiet, "quiet", false, "discard debug output")
	flag.BoolVar(&setup, "setup", true, "Pull")
	flag.Parse()

	dcli, err := client.NewEnvClient()
	if err != nil {
		log.Fatal(err)
	}
	cli = dcli
}

func setupDocker(conf *container.Config, hostConf *container.HostConfig, name string) (string, error) {
	ctx := context.Background()
	reader, err := cli.ImagePull(ctx, conf.Image, types.ImagePullOptions{})
	if err != nil {
		return "", err
	}

	io.Copy(os.Stdout, reader)

	resp, err := cli.ContainerCreate(ctx, conf, hostConf, nil, name)
	if err != nil {
		return "", err
	}

	containerIDS = append(containerIDS, resp.ID)

	return resp.ID, cli.ContainerStart(ctx, resp.ID, types.ContainerStartOptions{})
}

func tearDown() error {
	fmt.Println("tearing down containers")
	for _, id := range containerIDS {
		if err := cli.ContainerStop(context.Background(), id, nil); err != nil {
			return err
		}
	}

	return nil
}

func main() {
	if setup {
		hostBinding := nat.PortBinding{
			HostIP:   "localhost",
			HostPort: "9222",
		}
		containerPort, err := nat.NewPort("tcp", "9222")
		if err != nil {
			log.Fatal(err)
		}

		portBinding := nat.PortMap{containerPort: []nat.PortBinding{hostBinding}}
		chromeConf := &container.Config{
			Image: "docker.io/zenika/alpine-chrome",
			Cmd: strslice.StrSlice{
				"--headless", "--disable-gpu", "--no-sandbox", "--remote-debugging-address=0.0.0.0",
				"--remote-debugging-port=9222", "--enable-logging", "--autoplay-policy=no-user-gesture-required",
				"--disable-software-rasterizer", "--disable-dev-shm-usage", "--disable-sync",
				"--disable-background-networking", "--no-first-run", "--no-pings",
				"--metrics-recording-only", "--safebrowsing-disable-auto-update", "--mute-audio",
			},
			Entrypoint: strslice.StrSlice{"chromium-browser"},
		}
		chromeID, err := setupDocker(chromeConf,
			&container.HostConfig{
				PortBindings: portBinding,
				AutoRemove:   true,
			},
			"headless-chromium",
		)
		if err != nil {
			log.Fatal(err)
		}

		conJson, err := cli.ContainerInspect(context.Background(), chromeID)
		if err != nil {
			log.Fatal(err)
		}

		headlessURL = fmt.Sprintf("%s:%d", conJson.NetworkSettings.DefaultNetworkSettings.IPAddress, 9222)
		/*
			svcConf := &container.Config{}
			go func() {
				if err := setupDocker(svcConf, nil, ""); err != nil {
					log.Fatal(err)
				}
			}()
		*/
	}

	time.Sleep(3 * time.Second)

	hb, err := NewHeadlessBrowser(headlessURL, heuristics, quiet)
	if err != nil {
		log.Fatal(err)
	}

	resultChan := make(chan *network.Request, 100)
	go func() {
		err := hb.Run(target, time.Second*time.Duration(timeout), resultChan)
		if err != nil {
			log.Fatalf("FATAL: %q", err)
		}
	}()

	defer func() {
		if err := tearDown(); err != nil {
			log.Fatal(err)
		}
	}()

	select {}
}

type HeadlessBrowser struct {
	Info          *InstanceInfo
	stopChan      chan bool
	UseHeuristics bool
	Quiet         bool
}

// NewHeadlessBrowser connects to a headless browser at remote.
// If quiet is true, debug output is suppressed.
func NewHeadlessBrowser(remote string, useHeuristics bool, quiet bool) (*HeadlessBrowser, error) {
	ii, err := GetInstanceInfo(remote)
	if err != nil {
		return nil, fmt.Errorf("Unable to connect to instance: %q", err)
	}

	log.Printf("Found instace %q with User-Agent %q. Using debuggerURL %q.",
		ii.Browser,
		ii.UserAgent,
		ii.WebSocketDebuggerURL,
	)

	return &HeadlessBrowser{
		Info:          ii,
		stopChan:      make(chan bool, 1),
		UseHeuristics: useHeuristics,
		Quiet:         quiet,
	}, nil
}

// ExtractURL visits the given targetURL until it finds a new url that is accepted by matcherFunc or timeout expires.
func (hb *HeadlessBrowser) Run(targetURL string, timeout time.Duration, resultChan chan *network.Request) error {
	timeoutTicker := time.NewTicker(timeout)

	// source: https://github.com/chromedp/chromedp/blob/master/allocate_test.go
	allocCtx, allocCancel := chromedp.NewRemoteAllocator(context.Background(), hb.Info.WebSocketDebuggerURL)
	defer allocCancel()
	ctx, cancel := chromedp.NewContext(allocCtx)
	defer cancel()

	chromedp.ListenTarget(ctx, func(ev interface{}) {
		switch ev := ev.(type) {

		case *network.EventWebSocketCreated:
			if hb.Quiet {
				break
			}
			log.Printf("WEBSOCKET: %q", ev.URL)

		case *network.EventLoadingFailed:
			if hb.Quiet {
				break
			}
			log.Printf("FAILED: %q", ev.ErrorText)

		case *network.EventRequestWillBeSent:
			if hb.Quiet {
				break
			}
			log.Printf("REQUEST: %q", ev.Request.URL)

			/*
				url, err := url.Parse(ev.Request.URL)
				if err != nil {
					log.Printf("request %q error: %q", ev.Request.URL, err)
				}
				if url.String() != targetURL {
					// Navigation stalls if channel is blocked...
					go func() { resultChan <- ev.Request }()
				}
			*/
		}
	})

	if err := chromedp.Run(ctx,
		network.Enable(),             // enable network events
		chromedp.Navigate(targetURL), // navigate to url
	); err != nil {
		return err
	}

	log.Println("waiting for page to finish loading...")
	err := waitToFinishLoading(ctx, timeoutTicker)
	if err != nil {
		return err
	}

	select {
	case <-timeoutTicker.C:
		chromedp.Run(ctx,
			chromedp.Stop(),
		)
		return errors.New("timeout")
	case <-hb.stopChan:
		chromedp.Run(ctx,
			chromedp.Stop(),
		)
		log.Println("stopped!")
		return nil
	}
}

// Stop trys to abort ExtractURL and shuts down the headless browser instance.
func (hb *HeadlessBrowser) Stop() {
	hb.stopChan <- true
}

// waitToFinishLoading waits for site to finish loading (since clicking buttons mights not work correctly otherwise)
// source: https://github.com/chromedp/chromedp/issues/252
// Only returns with an error on timeout.
func waitToFinishLoading(ctx context.Context, timeoutTicker *time.Ticker) error {
	state := "notloaded"
	script := `document.readyState`
	checkTicker := time.NewTicker(time.Millisecond * 100)
	for {
		select {
		case <-checkTicker.C:
			err := chromedp.Run(ctx, chromedp.EvaluateAsDevTools(script, &state))
			if err != nil {
				log.Printf("error in eval: %q", err)
			}
			if strings.Compare(state, "complete") == 0 {
				return nil
			}
		case <-timeoutTicker.C:
			return errors.New("timeout while waiting to finish loading")
		}
	}
}
