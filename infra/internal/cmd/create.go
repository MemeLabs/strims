package cmd

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/internal/models"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/MemeLabs/go-ppspp/infra/pkg/wgutil"
	"github.com/spf13/cobra"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
)

func init() {
	rootCmd.AddCommand(createCmd)
}

var createCmd = &cobra.Command{
	Use:   "create [provider] [sku] [region]",
	Short: "Create node",
	Args: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		d, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("unsupported provider: %s", provider)
		}

		// TODO(jbpratt): refactor this since it will be used numerous times
		// across drivers
		regions, err := d.Regions(cmd.Context(), &node.RegionsRequest{})
		if err != nil {
			return fmt.Errorf("failed to get regions for current driver: %w", err)
		}

		// TODO(jbpratt): is validation needed here? Maybe, but only if the
		// implementations aren't already doing the validation otherwise, we
		// are just making duplicate requests..
		region := args[1]
		if !node.ValidRegion(region, regions) {
			return fmt.Errorf("invalid region for %q", provider)
		}

		skus, err := d.SKUs(cmd.Context(), &node.SKUsRequest{Region: region})
		if err != nil {
			return fmt.Errorf("failed to get skus for %q", provider)
		}

		sku := args[2]
		if !node.ValidSKU(sku, skus) {
			return fmt.Errorf("invalid sku for %q", provider)
		}

		return nil
	},
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		region := args[1]
		sku := args[2]
		d, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("invalid node provider for %q", provider)
		}

		req := &node.CreateRequest{
			Name:        generateHostname(provider, region),
			Region:      region,
			SKU:         sku,
			SSHKey:      backend.SSHPublicKey(),
			BillingType: node.Hourly,
		}
		n, err := d.Create(cmd.Context(), req)
		if err != nil {
			return fmt.Errorf("failed to create node(%v): %w", req, err)
		}

		jsonDump(n)
		tx, err := boil.BeginTx(cmd.Context(), nil)
		if err != nil {
			return err
		}

		slice, err := models.Nodes(qm.Where("active=?", 1)).All(cmd.Context(), tx)
		if err != nil {
			return err
		}

		for i := 0; i < len(slice); i++ {
			for _, peer := range backend.Conf.Peers {
				if peer.Endpoint == slice[i].IPV4 {
					continue
				}
			}

			backend.Conf.Peers = append(backend.Conf.Peers, wgutil.InterfacePeerConfig{
				PublicKey:           slice[i].WireguardKey,
				AllowedIPs:          slice[i].WireguardIP,
				Endpoint:            slice[i].IPV4,
				PersistentKeepalive: 25,
			})
		}

		wgIPv4, err := backend.NextWGIPv4(cmd.Context(), slice)
		if wgIPv4 == "" || err != nil {
			return fmt.Errorf("failed to get next wg ipv4: %w", err)
		}

		wgPriv, _, err := wgutil.GenerateKey()
		if wgPriv == "" || err != nil {
			return fmt.Errorf("failed to create wg keys: %w", err)
		}

		if err := backend.InsertNode(cmd.Context(), n, wgIPv4, wgPriv); err != nil {
			return fmt.Errorf("failed to insert node(%v): %w", n, err)
		}

		if err := backend.UpdateController(); err != nil {
			return fmt.Errorf("failed to update controller config(%v): %w", backend.Conf, err)
		}

		if err := backend.InitNode(
			cmd.Context(),
			n,
			d.DefaultUser(),
			wgIPv4,
		); err != nil {
			return fmt.Errorf("failed to init node(%v): %w", nil, err)
		}

		// TODO: controller should only have peers updated, not entire conf..
		if err := backend.SyncNodes(cmd.Context(), nil); err != nil {
			return fmt.Errorf("failed to sync nodes: %w", err)
		}

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

func generateHostname(provider, region string) string {
	name := make([]byte, 4)
	if _, err := rand.Read(name); err != nil {
		panic(err)
	}
	return fmt.Sprintf("%s-%s-%x", provider, region, name)
}
