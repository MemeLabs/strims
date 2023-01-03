package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
)

func main() {
	var (
		repo     = flag.String("registry", "ghcr.io/memelabs/strims", "container registry to publish image to")
		nodeip   = flag.String("node-ip", "", "node to execute mockstreamer from")
		username = flag.String("username", "ubuntu", "username of node for ssh")
		sshkey   = flag.String("ssh-key", os.Getenv("HOME")+"/.ssh/id_rsa", "ssh key")
	)
	flag.Parse()

	if *nodeip == "" {
		log.Fatal("must supply -node-ip flag")
	}

	if err := run(context.Background(), *repo, *username, *nodeip, *sshkey); err != nil {
		log.Fatal(err)
	}
}

func run(ctx context.Context, repo, username, nodeip, sshkey string) error {
	tag := "dev"
	image := fmt.Sprintf("%s/svc:%s", repo, tag)

	log.Println("building and publishing svc image")
	cmd := exec.CommandContext(ctx, "ko", "build", fmt.Sprintf("--tags=%s", tag), "--platform=linux/amd64,linux/arm64", "--base-import-paths", "./cmd/svc/")
	cmd.Env = append(os.Environ(), fmt.Sprintf("KO_DOCKER_REPO=%s", repo))
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("unable to build mockstreamer: %w", err)
	}

	log.Println("building mockstreamer binary")
	cmd = exec.CommandContext(ctx, "go", "build", "./infra/cmd/mockstreamer")
	output, err := cmd.CombinedOutput()
	if err != nil {
		fmt.Println(string(output))
		return err
	}

	log.Println("copying mockstreamer to test runner")
	cmd = exec.CommandContext(ctx, "scp", "-i", sshkey, "./mockstreamer", fmt.Sprintf("%s@%s:/tmp", username, nodeip))
	if err = cmd.Run(); err != nil {
		return fmt.Errorf("failed to scp binary to host: %w", err)
	}

	log.Println("executing mockstreamer")
	cmd = exec.CommandContext(ctx, "ssh", "-i", sshkey, fmt.Sprintf("%s@%s", username, nodeip), "/tmp/mockstreamer", "-image", image)
	if err = cmd.Run(); err != nil {
		return fmt.Errorf("failed to run mockstreamer: %w", err)
	}

	resultsDir, err := os.MkdirTemp("", "strims-logs-*")
	if err != nil {
		return err
	}

	log.Printf("fetching results back to %s", resultsDir)
	cmd = exec.CommandContext(ctx, "scp", "-i", sshkey, fmt.Sprintf("%s@%s:/tmp/results.tar.gz", username, nodeip), resultsDir)
	if err = cmd.Run(); err != nil {
		return err
	}

	return nil
}
