package cmd

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
)

func init() {
	createCmd.PersistentFlags().String("provider", "digitalocean", "hosting provider")
	createCmd.PersistentFlags().String("region", "sfo2", "hosting provider")

	rootCmd.AddCommand(createCmd)
}

var createCmd = &cobra.Command{
	Use:               "create",
	Short:             "Create node",
	Args:              cobra.ExactArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		d, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("Unsupported provider: %s", provider)
		}

		n, err := d.Create(context.Background(), &node.CreateRequest{
			Name:   "test",
			Region: "fr-par-1",
			SKU:    "DEV1-S",
			SSHKeys: []string{
				"ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQCwDB5GVPYZvxCncGsMRcC1ZLaWNDPlQnhsLSKxSwisg3n01XRGaX29h7WBWmuN3hc0YDDqgG5te3YGYwAALLHZ5Hm15rMzXdrmq8HxaknzbDNOrauwsgdXro/9t7PkZyDdDMOekZLn7/yatrsIUHTj0HGh9SQmLCsDjiX8CS+6k8+9kdBwxwt6l3ybDNxP2A+UeVQXnMfEBj96iG9o5hyrlKQYoD2oS+n9FQTw/Zr+McmdDz8XJz94ab4uCVCOBW1177q5kkNgHbe7oxKJyTisU/As85EONELcq//i3akGb7pygoCkaFELdUi6BessHy8TXKH6Nn3H6uXkNLhVpV/4mHCkyUdX14qjUg08aOjwiXbCNG4LmeehY1HOKjWqDh3NOAOGMIGObHr/M2rrHU5QLclX+Lr7LW1NGNergWCY6qi2zvsDoKRC8AXlGFqCmMig9pxaNZSPvHF7HhmX1f/mX7neri7X3ThzZmnb8PcsZMXCleVF6AQ/+0iyTSoyR1QDaGcJg0PmEdcD0ShqEP0RLXEGsG0OSAFy30BPRk0Q88iAkFsaHIM8msjXwIB2D0V/qzU+n6dfzs1Yx7in77admtKZYwvaDEkn9A5vo2EDwL+JrceLS4p2GOiw4E7sHbS4h6SMpyWcEVvr6hfGN3puJc/q/zLs9YGG/h5JmHZrUw== slugalisk@gmail.com",
			},
		})
		if err != nil {
			return err
		}

		jsonDump(n)

		// sshKeyPath := "/home/slugalisk/.ssh/id_rsa-slugalisk"

		// c := exec.Command("ssh", []string{
		// 	"-o", "UserKnownHostsFile=/dev/null",
		// 	"-o", "StrictHostKeyChecking=no",
		// 	"-i", sshKeyPath,
		// 	fmt.Sprintf("root@%s", n.Networks.V4[0]),
		// 	"touch ./test-file",
		// }...)

		// log.Println(c.String())

		// go relayStdio(c)

		// if err := c.Run(); err != nil {
		// 	return err
		// }

		return nil
	},
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}

func relayStdio(cmd *exec.Cmd) error {
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}

	copy := func(w io.Writer, r io.Reader) {
		if _, err := io.Copy(w, r); err != nil {
			panic(err)
		}
	}

	go copy(os.Stderr, stderr)
	go copy(os.Stdout, stdout)
	return nil
}
