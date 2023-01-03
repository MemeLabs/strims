package main

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
	"google.golang.org/protobuf/proto"
	v1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/serializer/yaml"
	"k8s.io/apimachinery/pkg/util/wait"
	yamlutil "k8s.io/apimachinery/pkg/util/yaml"
	"k8s.io/client-go/dynamic"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/restmapper"
	"k8s.io/client-go/tools/clientcmd"
	"k8s.io/client-go/util/homedir"
	"sigs.k8s.io/kustomize/api/krusty"
	"sigs.k8s.io/kustomize/kyaml/filesys"
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	var (
		outputDir    string
		image        string
		namespace    string
		nodeLabel    string
		controllerIP string
		metricsPort  int
		debugPort    int
		wsPort       int
		webrtcPort   int
		rtmpPort     int
		invitesPort  int
	)
	flag.StringVar(&outputDir, "output-dir", "/tmp", "directory to write results to")
	flag.StringVar(&namespace, "namespace", "strims", "kubernetes namespace svc is running in")
	flag.StringVar(&image, "image", "ghcr.io/memelabs/strims/svc:latest", "svc image to run")
	flag.StringVar(&nodeLabel, "node-label", "strims.gg/svc=seeder", "node label to run clients on")
	flag.StringVar(&controllerIP, "controller-ip", "10.0.0.1", "IP of the node exposing svc")
	flag.IntVar(&metricsPort, "metrics-port", 30000, "svc metrics port")
	flag.IntVar(&debugPort, "debug-port", 30001, "svc debug port")
	flag.IntVar(&wsPort, "ws-port", 30002, "svc websocket port")
	flag.IntVar(&webrtcPort, "webrtc-port", 30003, "svc webrtc port")
	flag.IntVar(&rtmpPort, "rtmp-port", 1935, "svc RTMP port")
	flag.IntVar(&invitesPort, "invites-port", 30005, "svc invites port")

	var kubeconfig string
	if home := homedir.HomeDir(); home != "" {
		flag.StringVar(&kubeconfig, "kubeconfig", filepath.Join(home, ".kube", "config"), "(optional) absolute path to the kubeconfig file")
	} else {
		flag.StringVar(&kubeconfig, "kubeconfig", "", "absolute path to the kubeconfig file")
	}
	flag.Parse()

	if kubeconfig == "" {
		kubeconfig = os.Getenv("KUBECONFIG")
	}

	config, err := clientcmd.BuildConfigFromFlags("", kubeconfig)
	if err != nil {
		return err
	}

	ctx := context.TODO()
	client := kubernetes.NewForConfigOrDie(config)

	log.Printf("installing using %s", image)
	if err = kustomize(ctx, client, config, namespace, image); err != nil {
		return err
	}

	nodesList, err := client.CoreV1().Nodes().List(ctx, metav1.ListOptions{LabelSelector: nodeLabel})
	if err != nil {
		return err
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		return err
	}

	host, err := newClient(logger, controllerIP, wsPort)
	if err != nil {
		return err
	}

	log.Println("host: signing up")
	signUpRequest := &authv1.SignUpRequest{Name: "host", Password: "password"}
	signUpResponse := &authv1.SignUpResponse{}
	if err = host.Auth.SignUp(ctx, signUpRequest, signUpResponse); err != nil {
		return fmt.Errorf("unable to create host profile: %w", err)
	}

	log.Println("host: creating server")
	createServerRequest := &networkv1.CreateServerRequest{Name: "test"}
	createServerResponse := &networkv1.CreateServerResponse{}
	if err = host.Network.CreateServer(ctx, createServerRequest, createServerResponse); err != nil {
		return fmt.Errorf("failed to create server: %w", err)
	}

	networkID := createServerResponse.GetNetwork().GetId()
	listPeersResponse := &networkv1.ListPeersResponse{}
	if err = host.Network.ListPeers(ctx, &networkv1.ListPeersRequest{NetworkId: networkID}, listPeersResponse); err != nil {
		return fmt.Errorf("error listing peers: %w", err)
	}

	log.Println("host: granting peer invitation for network")
	grantPeerInvitationReq := &networkv1.GrantPeerInvitationRequest{
		Id:    listPeersResponse.Peers[0].Id,
		Count: 1000000,
	}
	if err = host.Network.GrantPeerInvitation(ctx, grantPeerInvitationReq, &networkv1.GrantPeerInvitationResponse{}); err != nil {
		return fmt.Errorf("unable to grant peer invitation: %w", err)
	}

	log.Println("host: creating invitation request")
	createInvitationRequest := &networkv1.CreateInvitationRequest{NetworkId: networkID}
	createInvitationResponse := &networkv1.CreateInvitationResponse{}
	if err = host.Network.CreateInvitation(ctx, createInvitationRequest, createInvitationResponse); err != nil {
		return err
	}

	host.Close()

	profile := signUpResponse.GetProfile()
	invitation := createInvitationResponse.GetInvitation()

	var clients []*apis.FrontendClient

	eg := errgroup.Group{}
	for _, n := range nodesList.Items {
		node := n

		if !isNodeReady(&node) {
			log.Printf("node %s is not in a ready state, skipping", node.GetName())
			continue
		}

		eg.Go(func() error {
			client, err := newClient(logger, node.Status.Addresses[0].Address, wsPort)
			if err != nil {
				return err
			}
			clients = append(clients, client)

			log.Printf("%s: signing up", node.GetName())
			request := &authv1.SignUpRequest{Name: node.GetName(), Password: "password"}
			if err = client.Auth.SignUp(ctx, request, &authv1.SignUpResponse{}); err != nil {
				return err
			}

			log.Printf("%s: setting vnic limits", node.GetName())
			setVNICConfigReq := &vnicv1.SetConfigRequest{
				Config: &vnicv1.Config{
					MaxUploadBytesPerSecond: 1 << 40,
					MaxPeers:                500,
				},
			}
			if err = client.VNIC.SetConfig(ctx, setVNICConfigReq, &vnicv1.SetConfigResponse{}); err != nil {
				return err
			}

			log.Printf("%s: creating network from invitation", node.GetName())
			createNetworkFromInvitationReq := &networkv1.CreateNetworkFromInvitationRequest{
				Invitation: &networkv1.CreateNetworkFromInvitationRequest_InvitationBytes{
					InvitationBytes: errutil.Must(proto.Marshal(invitation)),
				},
			}
			if err = client.Network.CreateNetworkFromInvitation(ctx, createNetworkFromInvitationReq, &networkv1.CreateNetworkFromInvitationResponse{}); err != nil {
				return err
			}

			log.Printf("%s: creating bootstrap client", node.GetName())
			bootstrapClient := &networkv1bootstrap.CreateBootstrapClientRequest{
				ClientOptions: &networkv1bootstrap.CreateBootstrapClientRequest_WebsocketOptions{
					WebsocketOptions: &networkv1bootstrap.BootstrapClientWebSocketOptions{
						Url:                   fmt.Sprintf("ws://%s:%d/%x", controllerIP, wsPort, profile.GetKey().GetPublic()),
						InsecureSkipVerifyTls: true,
					},
				},
			}
			if err = client.Bootstrap.CreateClient(ctx, bootstrapClient, &networkv1bootstrap.CreateBootstrapClientResponse{}); err != nil {
				return err
			}

			debugConfigReq := &debugv1.SetConfigRequest{
				Config: &debugv1.Config{
					EnableMockStreams:    true,
					MockStreamNetworkKey: dao.NetworkKey(createServerResponse.Network),
				},
			}
			if err = client.Debug.SetConfig(ctx, debugConfigReq, &debugv1.SetConfigResponse{}); err != nil {
				return err
			}

			return nil
		})

	}
	if err = eg.Wait(); err != nil {
		return err
	}

	log.Println("starting mock stream")
	startMockStreamReq := &debugv1.StartMockStreamRequest{
		BitrateKbps:       6000,
		SegmentIntervalMs: 1000,
		TimeoutMs:         5 * 60 * 1000,
		NetworkKey:        dao.NetworkKey(createServerResponse.Network),
	}
	startMockStreamRes := &debugv1.StartMockStreamResponse{}
	if err = clients[0].Debug.StartMockStream(ctx, startMockStreamReq, startMockStreamRes); err != nil {
		return err
	}

	<-time.After(5 * time.Minute)

	log.Println("closing mock stream")
	stopMockStreamReq := &debugv1.StopMockStreamRequest{Id: startMockStreamRes.GetId()}
	if err = clients[0].Debug.StopMockStream(ctx, stopMockStreamReq, &debugv1.StopMockStreamResponse{}); err != nil {
		return err
	}

	for _, c := range clients {
		c.Close()
	}

	log.Println("gathering logs")
	svcPodsList, err := client.CoreV1().Pods(namespace).List(ctx, metav1.ListOptions{
		LabelSelector: "strims.gg/app in (leader,seeder)",
	})
	if err != nil {
		return err
	}

	archiveDir, err := os.MkdirTemp("", "strims-")
	if err != nil {
		return err
	}

	defer func() {
		_ = os.Remove(archiveDir)
	}()

	eg = errgroup.Group{}
	for _, p := range svcPodsList.Items {
		pod := p
		eg.Go(func() error {
			logs, err := getPodLogs(ctx, client, pod.GetName(), pod.GetNamespace())
			if err != nil {
				return err
			}
			return os.WriteFile(filepath.Join(archiveDir, fmt.Sprintf("%s-%s.log", pod.GetName(), pod.Spec.NodeName)), []byte(logs), 0o0644)
		})
	}

	if err = eg.Wait(); err != nil {
		return err
	}

	// download metrics

	resultsFile, err := os.Create(filepath.Join(outputDir, "results.tar.gz"))
	if err != nil {
		return err
	}
	defer resultsFile.Close()

	gzipper := gzip.NewWriter(resultsFile)
	defer gzipper.Close()
	tartar := tar.NewWriter(gzipper)
	defer tartar.Close()

	if err = filepath.WalkDir(archiveDir, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		if d.IsDir() {
			return nil
		}

		fileInfo, err := d.Info()
		if err != nil {
			return err
		}

		header, err := tar.FileInfoHeader(fileInfo, path)
		if err != nil {
			return err
		}

		// header.Name = filepath.ToSlash(path)
		if err = tartar.WriteHeader(header); err != nil {
			return err
		}

		data, err := os.Open(path)
		if err != nil {
			return err
		}

		if _, err = io.Copy(tartar, data); err != nil {
			return err
		}

		return data.Close()
	}); err != nil {
		return fmt.Errorf("failed archiving results: %w", err)
	}

	/*
		log.Println("tearing down install")
		if err = client.CoreV1().Namespaces().Delete(ctx, "strims", metav1.DeleteOptions{}); err != nil {
			return err
		}

		if err = client.RbacV1().ClusterRoleBindings().Delete(ctx, "strims-node-reader", metav1.DeleteOptions{}); err != nil {
			return err
		}

		if err = client.RbacV1().ClusterRoles().Delete(ctx, "strims-node-reader", metav1.DeleteOptions{}); err != nil {
			return err
		}
	*/

	return nil
}

func newClient(logger *zap.Logger, hostIP string, wsPort int) (*apis.FrontendClient, error) {
	c, resp, err := websocket.DefaultDialer.Dial(fmt.Sprintf("ws://%s:%d/api", hostIP, wsPort), nil)
	if err != nil {
		if resp != nil {
			return nil, fmt.Errorf("failed to dial with %d status code: %w", resp.StatusCode, err)
		} else {
			return nil, err
		}
	}

	rpcClient, err := rpc.NewClient(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: httputil.NewDefaultWSReadWriter(c),
	})
	if err != nil {
		return nil, fmt.Errorf("error creating rpc client: %w", err)
	}

	return apis.NewFrontendClient(rpcClient), nil
}

func getPodLogs(ctx context.Context, client kubernetes.Interface, name string, namespace string) (string, error) {
	req := client.CoreV1().Pods(namespace).GetLogs(name, &v1.PodLogOptions{})
	podLogs, err := req.Stream(ctx)
	if err != nil {
		return "", fmt.Errorf("unable to open pod %s log stream: %w", name, err)
	}
	defer podLogs.Close()

	buf := new(bytes.Buffer)
	_, err = io.Copy(buf, podLogs)
	if err != nil {
		return "", err
	}
	return buf.String(), nil
}

func isDeploymentReady(client kubernetes.Interface, name string, namespace string) wait.ConditionWithContextFunc {
	return func(ctx context.Context) (bool, error) {
		deployment, err := client.AppsV1().Deployments(namespace).Get(ctx, name, metav1.GetOptions{})
		if err != nil {
			return false, err
		}
		return deployment.Status.UpdatedReplicas == *deployment.Spec.Replicas, nil
	}
}

func isDaemonsetReady(client kubernetes.Interface, name string, namespace string) wait.ConditionWithContextFunc {
	return func(ctx context.Context) (bool, error) {
		daemonset, err := client.AppsV1().DaemonSets(namespace).Get(ctx, name, metav1.GetOptions{})
		if err != nil {
			return false, err
		}
		return daemonset.Status.NumberReady == daemonset.Status.DesiredNumberScheduled, nil
	}
}

func kustomize(ctx context.Context, client kubernetes.Interface, config *rest.Config, namespace, image string) error {
	imageParts := strings.Split(image, ":")
	if len(imageParts) != 2 {
		return fmt.Errorf("invalid image format, must be <repo>:<tag>")
	}

	kustomization := fmt.Sprintf(`
resources:
- https://github.com/MemeLabs/strims//infra/hack/kubernetes/strims/
images:
- name: ghcr.io/memelabs/strims/svc
  newName: %s
  newTag: %s`, imageParts[0], imageParts[1])

	fs := filesys.MakeFsOnDisk()
	tmpDir, err := filesys.NewTmpConfirmedDir()
	if err != nil {
		return err
	}
	defer func() {
		_ = os.Remove(tmpDir.String())
	}()

	if err = fs.WriteFile(tmpDir.Join("kustomization.yaml"), []byte(kustomization)); err != nil {
		return err
	}

	kustomizer := krusty.MakeKustomizer(krusty.MakeDefaultOptions())
	resources, err := kustomizer.Run(fs, tmpDir.String())
	if err != nil {
		return fmt.Errorf("unable to kustomize: %w", err)
	}

	bits, err := resources.AsYaml()
	if err != nil {
		return err
	}

	dynamicClient := dynamic.NewForConfigOrDie(config)
	decoder := yamlutil.NewYAMLOrJSONDecoder(bytes.NewReader(bits), 100)
	for {
		var rawObj runtime.RawExtension
		if err = decoder.Decode(&rawObj); err != nil {
			break
		}

		obj, gvk, err := yaml.NewDecodingSerializer(unstructured.UnstructuredJSONScheme).Decode(rawObj.Raw, nil, nil)
		if err != nil {
			return err
		}

		unstructuredMap, err := runtime.DefaultUnstructuredConverter.ToUnstructured(obj)
		if err != nil {
			return err
		}

		unstructuredObj := &unstructured.Unstructured{Object: unstructuredMap}
		gr, err := restmapper.GetAPIGroupResources(client.Discovery())
		if err != nil {
			return err
		}

		mapper := restmapper.NewDiscoveryRESTMapper(gr)
		mapping, err := mapper.RESTMapping(gvk.GroupKind(), gvk.Version)
		if err != nil {
			return err
		}

		// do i need any of this or does kustomize handle the namespace setting?
		var dri dynamic.ResourceInterface
		if mapping.Scope.Name() == meta.RESTScopeNameNamespace {
			if unstructuredObj.GetNamespace() == "" {
				unstructuredObj.SetNamespace(namespace)
			}
			dri = dynamicClient.Resource(mapping.Resource).Namespace(unstructuredObj.GetNamespace())
		} else {
			dri = dynamicClient.Resource(mapping.Resource)
		}

		if _, err = dri.Create(ctx, unstructuredObj, metav1.CreateOptions{}); err != nil {
			return err
		}
	}
	if !errors.Is(err, io.EOF) {
		return err
	}

	if err = wait.PollImmediateWithContext(ctx, time.Second, 30*time.Second, isDeploymentReady(client, "leader", namespace)); err != nil {
		return fmt.Errorf("timed out waiting for leader to be ready: %w", err)
	}

	if err = wait.PollImmediateWithContext(ctx, time.Second, 10*time.Minute, isDaemonsetReady(client, "seeder", namespace)); err != nil {
		return fmt.Errorf("timed out waiting for seeders to be ready: %w", err)
	}

	return nil
}

func isNodeReady(node *v1.Node) bool {
	for _, c := range node.Status.Conditions {
		if c.Type == v1.NodeReady {
			return c.Status == v1.ConditionTrue
		}
	}
	return false
}
