package cmd

import (
	"context"
	"fmt"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(destroyCmd)
}

var destroyCmd = &cobra.Command{
	Use:               "destroy",
	Short:             "Destroy node",
	Args:              cobra.ExactArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		d, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("Unsupported provider: %s", provider)
		}

		err := d.Delete(context.Background(), &node.DeleteRequest{
			Region:     "fr-par-1",
			ProviderID: "8026f443-6bc8-4aac-9441-4d33a2b6aa54",
		})
		if err != nil {
			return err
		}

		return nil
	},
}
