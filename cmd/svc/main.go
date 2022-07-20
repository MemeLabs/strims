// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"errors"
	"flag"
	"fmt"
	"log"
	"os"

	"golang.org/x/exp/slices"
)

func init() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)
}

func main() {
	code, err := run()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
	os.Exit(code)
}

func run() (int, error) {
	args := slices.Clone(os.Args)
	switch len(args) {
	case 0:
		return 1, errors.New("os args missing")
	case 1:
		args = append(args, "help")
	}

	cn := args[1]
	c, ok := commands[cn]
	if !ok {
		return 1, fmt.Errorf("invalid command '%s'", cn)
	}

	fs := c.Flags
	if fs == nil {
		fs = flag.NewFlagSet(cn, flag.ExitOnError)
	}

	err := fs.Parse(args[2:])
	if err != nil {
		return 1, err
	}

	err = c.Func(Flags{fs})
	if err != nil {
		return 1, fmt.Errorf("%s: %v", cn, err)
	}
	return 0, nil
}

type Flags struct {
	*flag.FlagSet
}

func (f Flags) String(name string) string {
	return f.Lookup(name).Value.String()
}
