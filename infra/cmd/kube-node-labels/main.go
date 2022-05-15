// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"context"
	"flag"
	"io/ioutil"
	"log"
	"path"
	"strings"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
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

	node, err := clientset.CoreV1().Nodes().Get(context.Background(), nodeName, metav1.GetOptions{})
	if err != nil {
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
