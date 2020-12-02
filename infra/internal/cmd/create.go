package cmd

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
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

		regions, err := d.Regions(cmd.Context(), &node.RegionsRequest{})
		if err != nil {
			return fmt.Errorf("failed to get regions for current driver: %w", err)
		}

		region := args[2]
		if !node.ValidRegion(region, regions) {
			return fmt.Errorf("invalid region for %q", provider)
		}

		skus, err := d.SKUs(cmd.Context(), &node.SKUsRequest{Region: region})
		if err != nil {
			return fmt.Errorf("failed to get skus for %q", provider)
		}

		sku := args[1]
		if !node.ValidSKU(sku, skus) {
			return fmt.Errorf("invalid sku for %q", provider)
		}

		return nil
	},
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {

		provider := args[0]
		sku := args[1]
		region := args[2]

		d, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("invalid node provider for %q", provider)
		}

		if err := backend.CreateNode(
			cmd.Context(),
			d,
			generateHostname(provider, region),
			region,
			sku,
			node.Hourly,
		); err != nil {
			return fmt.Errorf("failed to create node: %w", err)
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

func generateHostname(provider, region string) string {
	name := make([]byte, 4)
	if _, err := rand.Read(name); err != nil {
		panic(err)
	}
	return fmt.Sprintf("%s-%s-%x", provider, region, name)
}
