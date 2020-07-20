package driver

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"sync"
	"syscall"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/avast/retry-go"
	"github.com/chromedp/cdproto/network"
	"github.com/chromedp/cdproto/runtime"
	"github.com/chromedp/chromedp"
	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/api/types/filters"
	"github.com/docker/docker/api/types/strslice"
	"github.com/docker/docker/client"
	"github.com/docker/go-connections/nat"
	"github.com/gorilla/websocket"
)

const (
	chromeDebuggerPort = 9222

	// configured in webpack.config.js
	webpackDevServerPort = 8080
	testClientBridgePort = 8083
	testClientFilename   = "test.html"
)

// NewWeb ...
func NewWeb() (Driver, error) {
	d := &webDriver{}

	d.sig = make(chan os.Signal)
	signal.Notify(d.sig, syscall.SIGINT, syscall.SIGTERM)

	d.bridge = newTestClientBridgeServer()
	go func() {
		if err := d.bridge.Run(); err != nil {
			log.Println(err)
		}
	}()

	go func() {
		<-d.sig
		d.Close()
	}()

	return d, nil
}

type webDriver struct {
	sig       chan os.Signal
	bridge    *testClientBridgeServer
	clients   []webDriverClient
	closeOnce sync.Once
}

type webDriverClient struct {
	chrome    *chromeContainer
	devClient *devServerClient
}

// TODO: allow creating multiple clients
func (d *webDriver) Client(o *ClientOptions) *rpc.Client {
	chrome, err := newChromeContainer()
	if err != nil {
		log.Fatal(err)
	}

	devClient, err := newDevServerClient(chrome.Description.NetworkSettings.IPAddress)
	if err != nil {
		log.Fatal(err)
	}

	go func() {
		if err := devClient.Run(fmt.Sprintf(
			"https://%s:%d/%s",
			chrome.Description.NetworkSettings.Gateway,
			webpackDevServerPort,
			testClientFilename,
		)); err != nil {
			log.Fatal(err)
		}
		if err := chrome.Stop(); err != nil {
			log.Fatal(err)
		}
	}()

	d.clients = append(d.clients, webDriverClient{chrome, devClient})

	return <-d.bridge.Clients
}

func (d *webDriver) Close() {
	d.closeOnce.Do(func() {
		signal.Stop(d.sig)
		close(d.sig)
		d.bridge.Close()

		for _, c := range d.clients {
			c.devClient.Stop()
			if err := c.chrome.Stop(); err != nil {
				log.Println(err)
			}
		}
	})
}

func newTestClientBridgeServer() *testClientBridgeServer {
	return &testClientBridgeServer{
		Clients: make(chan *rpc.Client, 1),
	}
}

type testClientBridgeServer struct {
	upgrader websocket.Upgrader
	server   http.Server
	Clients  chan *rpc.Client
}

func (t *testClientBridgeServer) Run() error {
	t.server = http.Server{
		Addr:    fmt.Sprintf("0.0.0.0:%d", testClientBridgePort),
		Handler: http.HandlerFunc(t.handleRequest),
	}
	log.Println("starting server at", t.server.Addr)
	return t.server.ListenAndServe()
}

func (t *testClientBridgeServer) Close() {
	t.server.Close()
	close(t.Clients)
}

func (t *testClientBridgeServer) handleRequest(w http.ResponseWriter, r *http.Request) {
	c, err := t.upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Print("upgrade:", err)
		return
	}

	rw := vpn.NewWSReadWriter(c)
	client := rpc.NewClient(rw, rw)

	t.Clients <- client

	<-client.Done()
	rw.Close()
}

// chromeContainer ...
type chromeContainer struct {
	docker      *client.Client
	Description *types.ContainerJSON
}

// newChromeContainer ...
func newChromeContainer() (c *chromeContainer, err error) {
	c = &chromeContainer{}

	c.docker, err = client.NewEnvClient()
	if err != nil {
		return nil, err
	}

	hostBinding := nat.PortBinding{
		HostIP:   "localhost",
		HostPort: strconv.Itoa(chromeDebuggerPort),
	}
	containerPort, err := nat.NewPort("tcp", strconv.Itoa(chromeDebuggerPort))
	if err != nil {
		return nil, err
	}

	portBinding := nat.PortMap{containerPort: []nat.PortBinding{hostBinding}}
	containerConfig := &container.Config{
		Image:      "docker.io/zenika/alpine-chrome",
		Entrypoint: strslice.StrSlice{"chromium-browser"},
		Cmd: strslice.StrSlice{
			"--headless",
			"--disable-gpu",
			"--no-sandbox",
			"--remote-debugging-address=0.0.0.0",
			fmt.Sprintf("--remote-debugging-port=%d", chromeDebuggerPort),
			"--enable-logging",
			"--autoplay-policy=no-user-gesture-required",
			"--disable-software-rasterizer",
			"--disable-dev-shm-usage",
			"--disable-sync",
			"--disable-background-networking",
			"--no-first-run",
			"--no-pings",
			"--metrics-recording-only",
			"--safebrowsing-disable-auto-update",
			"--mute-audio",
			"--ignore-certificate-errors",
		},
	}

	if err := c.downloadImage(containerConfig.Image); err != nil {
		return nil, err
	}

	c.Description, err = c.runContainer(
		containerConfig,
		&container.HostConfig{
			PortBindings: portBinding,
			AutoRemove:   true,
		},
		fmt.Sprintf("headless-chromium-%d", time.Now().Unix()),
	)
	if err != nil {
		return nil, err
	}

	return c, nil
}

func (c *chromeContainer) downloadImage(ref string) error {
	f := filters.NewArgs()
	f.Add("reference", ref)
	images, err := c.docker.ImageList(context.Background(), types.ImageListOptions{Filters: f})
	if err != nil {
		return err
	}

	if len(images) == 0 {
		_, err := c.docker.ImagePull(context.Background(), ref, types.ImagePullOptions{})
		if err != nil {
			return err
		}
	}

	return nil
}

func (c *chromeContainer) runContainer(conf *container.Config, hostConf *container.HostConfig, name string) (*types.ContainerJSON, error) {
	resp, err := c.docker.ContainerCreate(context.Background(), conf, hostConf, nil, name)
	if err != nil {
		return nil, err
	}

	if err := c.docker.ContainerStart(context.Background(), resp.ID, types.ContainerStartOptions{}); err != nil {
		return nil, err
	}

	description, err := c.docker.ContainerInspect(context.Background(), resp.ID)
	if err != nil {
		return nil, err
	}

	return &description, nil
}

// Stop ...
func (c *chromeContainer) Stop() error {
	var timeout time.Duration
	return c.docker.ContainerStop(context.Background(), c.Description.ID, &timeout)
}

// devServerClient ...
type devServerClient struct {
	Info *chromeInstanceInfo
	stop chan struct{}
}

// newDevServerClient ...
func newDevServerClient(chromeDebuggerHost string) (*devServerClient, error) {
	b := &devServerClient{
		stop: make(chan struct{}),
	}

	err := retry.Do(func() (err error) {
		b.Info, err = getChromeInstanceInfo(fmt.Sprintf("%s:%d", chromeDebuggerHost, chromeDebuggerPort))
		return err
	})
	if err != nil {
		return nil, fmt.Errorf("Unable to connect to instance: %q", err)
	}

	log.Printf("Found instace %q with User-Agent %q. Using debuggerURL %q.",
		b.Info.Browser,
		b.Info.UserAgent,
		b.Info.WebSocketDebuggerURL,
	)

	return b, nil
}

// Run ...
func (b *devServerClient) Run(url string) error {
	allocCtx, allocCancel := chromedp.NewRemoteAllocator(context.Background(), b.Info.WebSocketDebuggerURL)
	defer allocCancel()

	ctx, cancel := chromedp.NewContext(allocCtx)
	defer cancel()

	done := make(chan error)

	chromedp.ListenTarget(ctx, func(ev interface{}) {
		switch ev := ev.(type) {
		case *network.EventWebSocketCreated:
			log.Printf("WEBSOCKET: %q", ev.URL)
		case *network.EventLoadingFailed:
			log.Printf("FAILED: %q", ev.ErrorText)
		case *network.EventRequestWillBeSent:
			log.Printf("REQUEST: %q", ev.Request.URL)
		case *runtime.EventConsoleAPICalled:
			fmt.Printf("* console.%s call:\n", ev.Type)
			for _, arg := range ev.Args {
				fmt.Printf("%s - %s\n", arg.Type, arg.Value)
			}
		case *runtime.EventExceptionThrown:
			if ev.ExceptionDetails.Exception.ClassName == "Success" {
				done <- nil
			} else {
				done <- fmt.Errorf("js exception: %s", ev.ExceptionDetails.Exception.Description)
			}
		}
	})

	if err := chromedp.Run(ctx, network.Enable(), chromedp.Navigate(url)); err != nil {
		return err
	}

	select {
	case <-b.stop:
		return errors.New("stop called")
	case err := <-done:
		return err
	}
}

// Stop ...
func (b *devServerClient) Stop() {
	close(b.stop)
}

// chromeInstanceInfo is the information that is provided by the debugger instance.
// By default, the information is exposed on http://localhost:9222/json/version
// reference: https://chromium.googlesource.com/external/github.com/mafredri/cdp/+/a974e2fd933e19fc0bbde4ea092df45158e782bf
type chromeInstanceInfo struct {
	Browser              string `json:"Browser"`
	ProtocolVersion      string `json:"Protocol-Version"`
	UserAgent            string `json:"User-Agent"`
	V8Version            string `json:"V8-Version"`
	WebKitVersion        string `json:"WebKit-Version"`
	WebSocketDebuggerURL string `json:"webSocketDebuggerUrl"`
}

// getChromeInstanceInfo fetches information about an instance running at the given endpoint.
// E.g. "localhost:9222".
func getChromeInstanceInfo(endpoint string) (*chromeInstanceInfo, error) {
	client := &http.Client{
		Timeout: time.Second * 3,
	}

	resp, err := client.Get(fmt.Sprintf("http://%s/json/version", endpoint))
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var ii chromeInstanceInfo
	if err = json.NewDecoder(resp.Body).Decode(&ii); err != nil {
		return nil, err
	}
	return &ii, err
}
