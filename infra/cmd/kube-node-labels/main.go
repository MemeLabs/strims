package main

import (
	"context"
	"flag"
	"io/ioutil"
	"log"
	"path"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
)

type stringsFlagValue []string

func (s *stringsFlagValue) String() string {
	return strings.Join(*s, ", ")
}

func (s *stringsFlagValue) Set(value string) error {
	*s = append(*s, value)
	return nil
}

func main() {
	var nodeName string
	var outputDir string
	var labelMaps stringsFlagValue

	flag.StringVar(&nodeName, "node-name", "", "kubernetes node name")
	flag.StringVar(&outputDir, "output-dir", "", "output directory")
	flag.Var(&labelMaps, "label", "label:filename pairs")
	flag.Parse()

	config, err := rest.InClusterConfig()
	if err != nil {
		log.Fatalf("getting in cluster k8s config: %v", err)
	}

	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		log.Fatalf("creating client for k8s config: %v", err)
	}

	nodeClient := clientset.NodeV1()
	res := nodeClient.RESTClient().Get().Name(nodeName).Do(context.Background())
	if res.Error() != nil {
		log.Fatalf("loading node: %v", res.Error())
	}

	var node corev1.Node
	if err := res.Into(&node); err != nil {
		log.Fatalf("loading node: %v", err)
	}

	for _, m := range labelMaps {
		l, p, ok := strings.Cut(m, ":")
		if !ok {
			log.Fatalf("malformed label map %s", m)
		}

		p = path.Join(outputDir, p)
		v := node.Labels[strings.TrimSpace(l)]

		log.Printf("writing label %s=%s to %s", l, v, p)

		err := ioutil.WriteFile(p, []byte(v), 0644)
		if err != nil {
			log.Fatalf("writing %s to %s: %v", l, p, err)
		}
	}
}
