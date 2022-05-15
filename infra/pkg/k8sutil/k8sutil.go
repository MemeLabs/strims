// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package k8sutil

// func newClientset() (*kubernetes.Clientset, error) {
// 	// config, err := rest.InClusterConfig()
// 	// if err != nil {
// 	// 	log.Println("failed in rest.InClusterConfig")
// 	// 	panic(err.Error())
// 	// }

// 	homeDir, err := os.UserHomeDir()
// 	if err != nil {
// 		return nil, err
// 	}

// 	configPath := path.Join(homeDir, ".kube", "config")
// 	config, err := clientcmd.BuildConfigFromFlags("", configPath)
// 	if err != nil {
// 		panic(err)
// 	}

// 	clientset, err := kubernetes.NewForConfig(config)
// 	if err != nil {
// 		log.Println("failed in kubernetes.NewForConfig")
// 		panic(err)
// 	}

// 	return clientset, nil
// }

// func CreateBootstrapToken() (string, error) {
// 	clientset, err := newClientset()
// 	if err != nil {
// 		return "", err
// 	}

// 	secret, err := clientset.CoreV1().Secrets("kube-system").Get("kubeadm-join-command", metav1.GetOptions{})
// 	if err != nil {
// 		return "", err
// 	}

// 	return string(secret.Data["kubeadm-join.sh"]), nil
// }

// `kubeadm token create --print-join-command | tr -d '\n' > /tmp/kubeadm-join.sh`

// CreateBootstrapToken ...
func CreateBootstrapToken() (string, error) {
	return "kubeadm join 10.0.0.1:6443 --token 53sw7y.cn0jm6tiwczfu0gi     --discovery-token-ca-cert-hash sha256:f476cecb5044265d7f4501d422ddf1af5324a9b13c9421164aded7e7a08b16fa", nil
}
