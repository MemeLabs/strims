package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(destroyCmd)
}

var destroyCmd = &cobra.Command{
	Use:               "destroy [name]",
	Short:             "Destroy node",
	Args:              cobra.ExactArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		name := args[0]
		// TODO: validate it is an active node

		if err := backend.DestroyNode(cmd.Context(), name); err != nil {
			return fmt.Errorf("failed to destroy node: %w", err)
		}

		return nil
	},
}
