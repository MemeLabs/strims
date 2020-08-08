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
			Region: "BHS5",
			SKU:    "S1-2",
			SSHKeys: []string{
				"ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQCqii+2B/KMkBtJOr0ku4wgbMrnuj5iVo5BmJGNzjdPmkLxQsOhZ3es0Gxb/1HJgOg1DptKPIxrMpWb1QCJf56zxUIcWKTHUIXzXY4KW0sT4bKSsE43AQQ0J2Ao3fQz8vdccWDPwpgrTaV6t1ZaFhb9sJJkzfplrBo2v0xVMSBieIpt4Znpi6HrIgXt6aqd5JpYuYv4SjYs/n+V2j62gAKKl7lt3ie+Nz50nrx9SPJ2+VrwCSQvidpGv1VY/tbG9j8VNff4fuxFl37au2TCfRYC7ANhTZjZWQOG3Yo920jziD+EY6lVv6G3GeMpVCny9lqcc+hUI+wP2Rd4Kw0RShwX1NrW7NyG+u8hjluIEubj4PWwwArMp6MQgdQKGhurOtBWBhdaFFrooiC4/DmAHUuPZAOK5vO0F1KEOUVVOz4VDsrU5Kw3X0NhBVcLDqrC9dwBMrqVBY5gnuboDb4Cq+RuW0cT9CIz2b7iwZZU8sg82O1Z2iu7qvER8TYJH4y8U2sE7OpkAfbOVqMlxW2x+O4ci6f9m8M/C7WRwGRoKvx422aaBihEJ7eQ5JXxlEwSWErbU+oXwdMRxJ6aMlWfXUCBhGfrTEa8sbyhThh9EsGvk+JV58EjfgqNqidDqSgzTh3Zfve0frj5rsS55CKK700pm8k/v/+sSl8tMC6oxGDdmw== jbpratt78@gmail.com",
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
