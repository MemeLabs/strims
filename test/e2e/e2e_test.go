package e2e

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/gorilla/websocket"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
	appsv1 "k8s.io/api/apps/v1"
	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/util/retry"
	"k8s.io/utils/pointer"
	"sigs.k8s.io/e2e-framework/klient/k8s/resources"
	"sigs.k8s.io/e2e-framework/klient/wait"
	"sigs.k8s.io/e2e-framework/klient/wait/conditions"
	"sigs.k8s.io/e2e-framework/pkg/env"
	"sigs.k8s.io/e2e-framework/pkg/envconf"
	"sigs.k8s.io/e2e-framework/pkg/envfuncs"
	"sigs.k8s.io/e2e-framework/pkg/features"
)

type contextKey int

const (
	prefix         = "strims-e2e"
	kindConfigPath = "testdata/kind-config.yaml"
	defaultImage   = "ghcr.io/memelabs/strims/svc:latest"

	manifestsDirPath = "testdata/manifests"
	testVideoPath    = "../../pkg/rtmpingress/testdata/sample.mp4"

	contextKeyNetwork contextKey = iota
	contextKeyFrontendClient
	contextKeyRPCClient
	contextKeyChannel
	contextKeyURL
)

var (
	testenv env.Environment

	testImage             string
	testNamespace         string
	controllerIP          string
	metricsPort           int
	debugPort             int
	wsPort                int
	webrtcPort            int
	rtmpPort              int
	invitesPort           int
	useExistingDeployment bool

	originalImage string
)

func TestMain(m *testing.M) {
	flag.StringVar(&testImage, "strims.image", "", "svc container image")
	flag.StringVar(&testNamespace, "strims.namespace", "", "test namespace to use")
	flag.StringVar(&controllerIP, "strims.controller-ip", "10.0.0.1", "IP of the node exposing svc")
	flag.IntVar(&metricsPort, "strims.metrics-port", 30000, "svc metrics port")
	flag.IntVar(&debugPort, "strims.debug-port", 30001, "svc debug port")
	flag.IntVar(&wsPort, "strims.ws-port", 30002, "svc websocket port")
	flag.IntVar(&webrtcPort, "strims.webrtc-port", 30003, "svc webrtc port")
	flag.IntVar(&rtmpPort, "strims.rtmp-port", 1935, "svc RTMP port")
	flag.IntVar(&invitesPort, "strims.invites-port", 30005, "svc invites port")
	flag.BoolVar(&useExistingDeployment, "strims.use-existing-deployment", false, "utilize an existing deployment of svc")

	cfg, err := envconf.NewFromFlags()
	if err != nil {
		log.Fatalf("envconf failed: %s", err)
	}

	testenv = env.NewWithConfig(cfg)

	if useExistingDeployment && testNamespace == "" {
		log.Fatal("must supply a test namespace when using an existing deployment")
	}

	setupFuncs := []env.Func{}
	finishFuncs := []env.Func{}

	if useExistingDeployment {
		if testImage != "" {
			setupFuncs = append(setupFuncs, configureExistingDeployment())
			finishFuncs = append(finishFuncs, resetExistingDeployment())
		}
	} else {
		controllerIP = "localhost"

		// TODO: generate cluster name
		kindClusterName := prefix
		setupFuncs = append(setupFuncs, envfuncs.CreateKindClusterWithConfig(kindClusterName, "\"\"", kindConfigPath))

		if testImage != "" {
			if strings.HasSuffix(testImage, ".tar") {
				setupFuncs = append(setupFuncs, envfuncs.LoadImageArchiveToCluster(kindClusterName, testImage))
			} else {
				setupFuncs = append(setupFuncs, envfuncs.LoadDockerImageToCluster(kindClusterName, testImage))
			}
		} else {
			testImage = defaultImage
		}

		if testNamespace == "" {
			testNamespace = envconf.RandomName(prefix, 15)
			setupFuncs = append(setupFuncs, envfuncs.CreateNamespace(testNamespace))
			finishFuncs = append(finishFuncs, envfuncs.DeleteNamespace(testNamespace))
		}

		setupFuncs = append(setupFuncs, createNewDeployment())
		finishFuncs = append(finishFuncs, teardownNewDeployment(), envfuncs.DestroyKindCluster(kindClusterName))
	}

	testenv.Setup(setupFuncs...)
	testenv.Finish(finishFuncs...)

	os.Exit(testenv.Run(m))

}

func configureExistingDeployment() env.Func {
	return func(ctx context.Context, conf *envconf.Config) (context.Context, error) {
		client, err := conf.NewClient()
		if err != nil {
			return ctx, err
		}

		r, err := resources.New(client.RESTConfig())
		if err != nil {
			return ctx, err
		}

		deployment := &appsv1.Deployment{}
		if err = retry.RetryOnConflict(retry.DefaultRetry, func() error {
			if err = r.Get(ctx, "leader", testNamespace, deployment); err != nil {
				return err
			}

			originalImage = deployment.Spec.Template.Spec.Containers[0].Image
			deployment.Spec.Template.Spec.Containers[0].Image = testImage

			return r.Update(ctx, deployment)
		}); err != nil {
			return ctx, fmt.Errorf("failed to patch deployment: %w", err)
		}

		deploymentReady := conditions.New(r).DeploymentConditionMatch(deployment, appsv1.DeploymentAvailable, v1.ConditionTrue)
		if err = wait.For(deploymentReady); err != nil {
			return ctx, err
		}

		return ctx, nil
	}
}

func resetExistingDeployment() env.Func {
	return func(ctx context.Context, conf *envconf.Config) (context.Context, error) {
		client, err := conf.NewClient()
		if err != nil {
			return ctx, err
		}

		r, err := resources.New(client.RESTConfig())
		if err != nil {
			return ctx, err
		}

		deployment := &appsv1.Deployment{}
		if err = r.Get(ctx, "leader", testNamespace, deployment); err != nil {
			return ctx, fmt.Errorf("failed to find deployment: %w", err)
		}

		if originalImage != "" {
			deployment.Spec.Template.Spec.Containers[0].Image = originalImage
		}

		if err = r.Update(ctx, deployment); err != nil {
			return ctx, fmt.Errorf("failed to patch deployment image: %w", err)
		}

		return ctx, nil
	}
}

func createNewDeployment() env.Func {
	return func(ctx context.Context, conf *envconf.Config) (context.Context, error) {
		client, err := conf.NewClient()
		if err != nil {
			return ctx, err
		}

		r, err := resources.New(client.RESTConfig())
		if err != nil {
			return ctx, err
		}

		config := fmt.Sprintf(`metrics:
  address: 0.0.0.0:%d
debug:
  address: 0.0.0.0:%d
storage:
  adapter: bbolt
  bbolt:
    path: /bbolt/.strims
http:
  address: 0.0.0.0:%d
session:
  remote:
    enabled: true
  headless: []
vnic:
  webrtc:
    tcpMuxAddress: 0.0.0.0:%d
    udpMuxAddress: 0.0.0.0:%d
`, metricsPort, debugPort, wsPort, webrtcPort, webrtcPort)

		configmap := &v1.ConfigMap{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "leader",
				Namespace: testNamespace,
			},
			Data: map[string]string{
				"config.yaml": config,
			},
		}
		if err = r.Create(ctx, configmap); err != nil {
			return ctx, err
		}

		deployment := &appsv1.Deployment{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "leader",
				Namespace: testNamespace,
			},
			Spec: appsv1.DeploymentSpec{
				Replicas: pointer.Int32(1),
				Selector: &metav1.LabelSelector{
					MatchLabels: map[string]string{"strims.gg/app": "svc"},
				},
				Template: v1.PodTemplateSpec{
					ObjectMeta: metav1.ObjectMeta{
						Labels: map[string]string{"strims.gg/app": "svc"},
					},
					Spec: v1.PodSpec{
						NodeSelector: map[string]string{"strims.gg/svc": "leader"},
						Containers: []v1.Container{
							{
								Name:  "strims",
								Image: testImage,
								Args:  []string{"run", "-config", "/etc/strims/config.yaml"},
								Ports: []v1.ContainerPort{
									{Name: "metrics", ContainerPort: int32(metricsPort)},
									{Name: "debug", ContainerPort: int32(debugPort)},
									{Name: "http", ContainerPort: int32(wsPort), HostPort: int32(wsPort)},
									{Name: "webrtc-tcp", ContainerPort: int32(webrtcPort), HostPort: int32(webrtcPort)},
									{Name: "webrtc-udp", ContainerPort: int32(webrtcPort), HostPort: int32(webrtcPort), Protocol: v1.ProtocolUDP},
									{Name: "rtmp", ContainerPort: int32(rtmpPort), HostPort: int32(rtmpPort)},
								},
								VolumeMounts: []v1.VolumeMount{
									{Name: "config-vol", MountPath: "/etc/strims/"},
									{Name: "database-vol", MountPath: "/bbolt/"},
								},
							},
						},
						Volumes: []v1.Volume{
							{
								Name: "config-vol",
								VolumeSource: v1.VolumeSource{
									ConfigMap: &v1.ConfigMapVolumeSource{LocalObjectReference: v1.LocalObjectReference{Name: configmap.GetName()}},
								},
							},
							{
								Name:         "database-vol",
								VolumeSource: v1.VolumeSource{EmptyDir: &v1.EmptyDirVolumeSource{}},
							},
						},
					},
				},
			},
		}
		if err = r.Create(ctx, deployment); err != nil {
			return ctx, err
		}

		deploymentReady := conditions.New(r).DeploymentConditionMatch(deployment, appsv1.DeploymentAvailable, v1.ConditionTrue)
		if err = wait.For(deploymentReady); err != nil {
			return ctx, err
		}

		return ctx, nil
	}
}

func teardownNewDeployment() env.Func {
	return func(ctx context.Context, conf *envconf.Config) (context.Context, error) {
		client, err := conf.NewClient()
		if err != nil {
			return ctx, err
		}

		r, err := resources.New(client.RESTConfig())
		if err != nil {
			return ctx, err
		}

		deployment := &appsv1.Deployment{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "leader",
				Namespace: testNamespace,
			},
		}
		if err := r.Delete(ctx, deployment); err != nil {
			return ctx, err
		}

		return ctx, nil
	}
}

func TestSimpleStreaming(t *testing.T) {
	streaming := features.New("streaming").
		WithSetup("setup api client", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			c, resp, err := websocket.DefaultDialer.Dial(fmt.Sprintf("ws://%s:%d/api", controllerIP, wsPort), nil)
			require.NoError(t, err, "unable to dial ws server with response: %v", resp)

			// TODO: using the zaptest logger throws a panic on complete due to
			// logging in the rpc.NewClient func1() goroutine
			// logger := zaptest.NewLogger(t, zaptest.WrapOptions(zap.AddCaller()))
			logger, err := zap.NewDevelopment()
			require.NoError(t, err)

			client, err := rpc.NewClient(logger, &rpc.RWDialer{
				Logger:     logger,
				ReadWriter: httputil.NewDefaultWSReadWriter(c),
			})
			require.NoError(t, err)
			ctx = context.WithValue(ctx, contextKeyRPCClient, client)

			return context.WithValue(ctx, contextKeyFrontendClient, apis.NewFrontendClient(client))
		}).
		WithSetup("prerequisites exist", func(ctx context.Context, t *testing.T, c *envconf.Config) context.Context {
			require.FileExists(t, testVideoPath)
			_, err := exec.LookPath("ffmpeg")
			require.NoError(t, err, "ffmpeg is not in $PATH")
			return ctx
		}).
		Assess("create a profile", func(ctx context.Context, t *testing.T, c *envconf.Config) context.Context {
			request := &authv1.SignUpRequest{Name: envconf.RandomName("dev", 10), Password: envconf.RandomName("pw", 15)}
			err := client(ctx).Auth.SignUp(ctx, request, &authv1.SignUpResponse{})
			require.NoError(t, err)
			return ctx
		}).
		Assess("create a network", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			request := &networkv1.CreateServerRequest{Name: envconf.RandomName("test", 10)}
			response := &networkv1.CreateServerResponse{}
			err := client(ctx).Network.CreateServer(ctx, request, response)
			require.NoError(t, err)
			return context.WithValue(ctx, contextKeyNetwork, response.Network)
		}).
		Assess("create a channel", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			request := &videov1.VideoChannelCreateRequest{
				DirectoryListingSnippet: &networkv1directory.ListingSnippet{
					Title:       "test stream",
					Description: "wow, such testing",
					Tags:        []string{"NSFW"},
					Category:    "test",
					ChannelName: "tester",
					IsMature:    true,
				},
				NetworkKey: dao.NetworkKey(ctx.Value(contextKeyNetwork).(*networkv1.Network)),
			}
			response := &videov1.VideoChannelCreateResponse{}
			err := client(ctx).VideoChannel.Create(ctx, request, response)
			require.NoError(t, err)
			return context.WithValue(ctx, contextKeyChannel, response.Channel)
		}).
		Assess("enable video ingress", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			request := &videov1.VideoIngressSetConfigRequest{
				Config: &videov1.VideoIngressConfig{
					Enabled:          true,
					ServerAddr:       fmt.Sprintf("0.0.0.0:%d", rtmpPort),
					PublicServerAddr: fmt.Sprintf("%s:%d", controllerIP, rtmpPort),
				},
			}
			response := &videov1.VideoIngressSetConfigResponse{}
			err := client(ctx).VideoIngress.SetConfig(ctx, request, response)
			require.NoError(t, err)
			return ctx
		}).
		Assess("get channel url", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			channel := ctx.Value(contextKeyChannel).(*videov1.VideoChannel)
			request := &videov1.VideoIngressGetChannelURLRequest{Id: channel.Id}
			response := &videov1.VideoIngressGetChannelURLResponse{}
			err := client(ctx).VideoIngress.GetChannelURL(ctx, request, response)
			require.NoError(t, err)
			return context.WithValue(ctx, contextKeyURL, response.Url)
		}).
		Assess("stream with rtmp", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			ctxG, cancel := context.WithCancel(ctx)
			eg, ctxG := errgroup.WithContext(ctxG)

			eg.Go(func() error {
				cmd := exec.CommandContext(
					ctxG,
					"ffmpeg",
					"-hide_banner",
					"-loglevel", "warning",
					"-re",
					"-i", testVideoPath,
					"-c:v", "libx264",
					"-pix_fmt", "yuv420p",
					"-g", "24",
					"-keyint_min", "24",
					"-b:v", "6000k",
					"-maxrate", "6000k",
					"-c:a", "aac",
					"-strict", "-2",
					"-ar", "44100",
					"-b:a", "160k",
					"-ac", "2",
					"-bufsize", "3000k",
					"-flvflags", "no_duration_filesize",
					"-f", "flv",
					ctxG.Value(contextKeyURL).(string),
				)

				// TODO: if debug
				cmd.Stdout = os.Stdout
				cmd.Stderr = os.Stderr
				if err := cmd.Run(); err != nil {
					return err
				}

				cancel()
				return nil
			})

			var listings []*networkv1directory.NetworkListings
			require.Eventuallyf(t, func() bool {
				request := &networkv1directory.FrontendGetListingsRequest{
					ContentTypes: []networkv1directory.ListingContentType{
						networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_MEDIA,
					},
				}
				response := &networkv1directory.FrontendGetListingsResponse{}
				err := client(ctxG).Directory.GetListings(ctxG, request, response)
				require.NoError(t, err)

				listings = response.Listings
				return len(listings) == 1 && len(listings[0].Listings) == 1
			}, 10*time.Second, 1*time.Second, "network listing never appeard")

			request := &videov1.EgressOpenStreamRequest{
				SwarmUri:    listings[0].Listings[0].Listing.Content.(*networkv1directory.Listing_Media_).Media.SwarmUri,
				NetworkKeys: [][]byte{},
			}
			response := make(chan *videov1.EgressOpenStreamResponse)
			eg.Go(func() error {
				err := client(ctxG).VideoEgress.OpenStream(ctxG, request, response)
				if errors.Is(err, context.Canceled) {
					return nil
				}
				return err
			})

			var readBytes uint64
			eg.Go(func() error {
				for e := range response {
					switch m := e.Body.(type) {
					case *videov1.EgressOpenStreamResponse_Open_:
						t.Log("stream opened...")
					case *videov1.EgressOpenStreamResponse_Data_:
						readBytes += uint64(len(m.Data.Data))
					case *videov1.EgressOpenStreamResponse_Error_:
						return nil
					}
				}
				return nil
			})

			err := eg.Wait()
			require.NoError(t, err, "failed to stream video")
			require.Greater(t, readBytes, uint64(0), "no bytes were read")

			return ctx
		}).
		WithTeardown("disable video ingress", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			request := &videov1.VideoIngressSetConfigRequest{Config: &videov1.VideoIngressConfig{Enabled: false}}
			err := client(ctx).VideoIngress.SetConfig(ctx, request, &videov1.VideoIngressSetConfigResponse{})
			require.NoError(t, err)
			return ctx
		}).
		WithTeardown("delete the channel", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			channel := ctx.Value(contextKeyChannel).(*videov1.VideoChannel)
			request := &videov1.VideoChannelDeleteRequest{Id: channel.GetId()}
			err := client(ctx).VideoChannel.Delete(ctx, request, &videov1.VideoChannelDeleteResponse{})
			require.NoError(t, err)
			return ctx
		}).
		WithTeardown("delete the network", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			network := ctx.Value(contextKeyNetwork).(*networkv1.Network)
			request := &networkv1.DeleteNetworkRequest{Id: network.GetId()}
			err := client(ctx).Network.Delete(ctx, request, &networkv1.DeleteNetworkResponse{})
			require.NoError(t, err)
			return ctx
		}).
		WithTeardown("close api client", func(ctx context.Context, t *testing.T, _ *envconf.Config) context.Context {
			ctx.Value(contextKeyRPCClient).(*rpc.Client).Close()
			return ctx
		}).
		Feature()

	testenv.Test(t, streaming)
}

func TestMockStreamSwarm(t *testing.T) {}

func client(ctx context.Context) *apis.FrontendClient {
	return ctx.Value(contextKeyFrontendClient).(*apis.FrontendClient)
}
