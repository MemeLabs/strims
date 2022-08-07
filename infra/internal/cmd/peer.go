// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	addPeerCmd.Flags().StringP("address", "a", "", "external IP address")
	addPeerCmd.Flags().IntP("port", "p", 51820, "WireGuard listening port")
	addPeerCmd.MarkFlagRequired("address")

	// TODO: list peers
	peerCmd.AddCommand(addPeerCmd)
	peerCmd.AddCommand(removePeerCmd)
	peerCmd.AddCommand(configPeerCmd)
	rootCmd.AddCommand(peerCmd)
}

var peerCmd = &cobra.Command{
	Use:   "peer",
	Short: "Manage external WireGuard peers",
}

var addPeerCmd = &cobra.Command{
	Use:   "add [name]",
	Short: "Add an external peer by name",
	Args:  cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		address, _ := cmd.Flags().GetString("address")
		port, err := cmd.Flags().GetInt("port")
		if err != nil {
			return err
		}

		conf, err := backend.AddStaticPeer(cmd.Context(), args[0], address, port)
		if err != nil {
			return err
		}

		fmt.Println(conf.String())
		return nil
	},
}

var removePeerCmd = &cobra.Command{
	Aliases: []string{"delete"},
	Use:     "remove [name]",
	Short:   "Remove an external peer by name",
	Args:    cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		return backend.RemoveStaticPeer(cmd.Context(), args[0])
	},
}

var configPeerCmd = &cobra.Command{
	Aliases: []string{"get"},
	Use:     "config [name]",
	Short:   "Get the WireGuard config for a specific peer by name",
	Args:    cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		conf, err := backend.GetConfigForPeer(cmd.Context(), args[0])
		if err != nil {
			return err
		}

		fmt.Println(conf.String())
		return nil
	},
}
