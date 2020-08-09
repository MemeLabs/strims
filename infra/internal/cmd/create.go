package cmd

import (
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

		n, err := d.Create(cmd.Context(), &node.CreateRequest{
			Name:   "test",
			Region: "RegionOne",
			SKU:    "100",
			SSHKey: backend.SSHPublicKey(),
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
