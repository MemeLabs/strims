// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"errors"
	"flag"
)

type Command struct {
	Name  string
	Func  CommandFunc
	Usage string
	Short string
	Long  string
	Flags *flag.FlagSet
}

type CommandFunc func(Flags) error

var commands = map[string]Command{}

func RegisterCommand(c Command) {
	commands[c.Name] = c
}

func noopCmd(fs Flags) error {
	return errors.New("not implemented")
}

func init() {
	RegisterCommand(Command{
		Name: "help",
		Func: noopCmd,
	})

	RegisterCommand(Command{
		Name:  "run",
		Func:  runCmd,
		Usage: "[--config <path>] [--host-ip <ip>]",
		Short: `Starts the peer server`,
		Flags: func() *flag.FlagSet {
			fs := flag.NewFlagSet("run", flag.ExitOnError)
			fs.String("config", "", "Configuration file")
			fs.String("host-ip", "", "Public IP address")
			fs.String("public-hostname", "", "Public domain name")
			return fs
		}(),
	})

	RegisterCommand(Command{
		Name: "list-profiles",
		Func: noopCmd,
	})

	RegisterCommand(Command{
		Name:  "add-profile",
		Func:  addProfileCmd,
		Usage: "[--config <path>] --username <string> --password <string>",
		Short: `Adds a new profile to the db`,
		Flags: func() *flag.FlagSet {
			fs := flag.NewFlagSet("run", flag.ExitOnError)
			fs.String("config", "", "Configuration file")
			fs.String("username", "", "Profile username")
			fs.String("password", "", "Profile password")
			fs.Bool("json", false, "Print output as json")
			return fs
		}(),
	})

	RegisterCommand(Command{
		Name: "remove-profile",
		Func: noopCmd,
	})

	RegisterCommand(Command{
		Name: "import-profile",
		Func: noopCmd,
	})

	RegisterCommand(Command{
		Name: "export-profile",
		Func: noopCmd,
	})

	RegisterCommand(Command{
		Name:  "serve-invites",
		Func:  serveInvitesCmd,
		Usage: "[--config <path>]",
		Short: `Starts the invitation code server`,
		Flags: func() *flag.FlagSet {
			fs := flag.NewFlagSet("run", flag.ExitOnError)
			fs.String("config", "", "Configuration file")
			return fs
		}(),
	})
}
