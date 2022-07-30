// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	addPeerCmd.Flags().StringP("address", "a", "", "external IP address")
	addPeerCmd.Flags().IntP("port", "p", 51820, "WireGuard listening port")
	addPeerCmd.MarkFlagRequired("address")

	peerCmd.PersistentFlags().StringP("name", "n", "", "name of peer")
	peerCmd.MarkFlagRequired("name")

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
	Use:   "add",
	Short: "Add an external peer",
	RunE: func(cmd *cobra.Command, _ []string) error {
		name, _ := cmd.Flags().GetString("name")
		address, _ := cmd.Flags().GetString("address")
		port, err := cmd.Flags().GetInt("port")
		if err != nil {
			return err
		}

		conf, err := backend.AddStaticPeer(context.TODO(), name, address, port)
		if err != nil {
			return err
		}

		fmt.Println(conf.String())
		return nil
	},
}

var removePeerCmd = &cobra.Command{
	Aliases: []string{"delete"},
	Use:     "remove",
	Short:   "Remove an external peer",
	RunE: func(cmd *cobra.Command, _ []string) error {
		name, _ := cmd.Flags().GetString("name")
		return backend.RemoveStaticPeer(context.TODO(), name)
	},
}

var configPeerCmd = &cobra.Command{
	Aliases: []string{"get"},
	Use:     "config",
	Short:   "Get the WireGuard config for a specific peer",
	RunE: func(cmd *cobra.Command, _ []string) error {
		name, _ := cmd.Flags().GetString("name")
		conf, err := backend.GetConfigForPeer(context.TODO(), name)
		if err != nil {
			return err
		}

		fmt.Println(conf.String())
		return nil
	},
}
