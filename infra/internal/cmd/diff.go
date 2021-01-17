package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(diffCmd)
}

var diffCmd = &cobra.Command{
	Use:   "diff",
	Short: "Find differences between active instances and the database",
	Args:  cobra.NoArgs,
	RunE: func(cmd *cobra.Command, args []string) error {
		diff, err := backend.DiffNodes(cmd.Context())
		if err != nil {
			return fmt.Errorf("failed to diff nodes: %w", err)
		}

		fmt.Println(diff)

		return nil
	},
}
