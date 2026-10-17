// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// internal/directory on the node, for the directory interop tests. With
// -directory (and -network-control), internal/directory's control runs on
// the helper's profile store, beside the network control and its transfer
// control, as internal/app builds it: it serves the directory of the
// networks the profile owns and reads the others', and serves snippets on
// the snippet port of every network. The event broadcasts and asset
// bundles its readers read are reported as dir-event and dir-assets lines
// (see network.go).
//
// Verbs (byte strings in hex, <key> a network key):
//
//	dir-publish <key> <listing>                       publish a Listing -> <id>
//	dir-unpublish <key> <id>
//	dir-join <key> <query>                            view the listing a ListingQuery names -> <id>
//	dir-part <key> <id>
//	dir-moderate-listing <key> <id> <moderation>      a ListingModeration, or - for none
//	dir-moderate-user <key> <peer key> <moderation>   a UserModeration, or - for none
//	dir-listings <network id>                         the view's listings -> <NetworkListings>
//	dir-users <network id>                            the view's users -> <id>:<alias>:<peer key>,... or -
//	dir-snippet <key> <snippet>                       a new writer swarm, its transfer under the empty
//	                                                  salt published on the network, serving the
//	                                                  ListingSnippet -> <swarm uri>
//	dir-snippet-update <uri> <snippet>                serve another snippet for the swarm
//	dir-config <network id> <config>                  set an owned network's directory ServerConfig
//
// The snippets of dir-snippet and dir-snippet-update are signed with the
// swarm's key unless they come signed. A call that fails answers err with
// the error's message.

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"google.golang.org/protobuf/proto"
)

// directoryCallTimeout bounds each call to a directory server.
const directoryCallTimeout = 20 * time.Second

func (n *node) directoryCommand(verb string, args []string) (string, error) {
	s := n.network
	if s == nil || s.directory == nil {
		return "", fmt.Errorf("%s: the directory is off", verb)
	}
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	bytesArg := func(i int) ([]byte, error) {
		if err := need(i + 1); err != nil {
			return nil, err
		}
		return unhex(args[i])
	}
	uintArg := func(i int) (uint64, error) {
		if err := need(i + 1); err != nil {
			return 0, err
		}
		return strconv.ParseUint(args[i], 10, 64)
	}
	message := func(i int, m proto.Message) error {
		b, err := bytesArg(i)
		if err != nil {
			return err
		}
		return proto.Unmarshal(b, m)
	}
	ctx, cancel := context.WithTimeout(context.Background(), directoryCallTimeout)
	defer cancel()
	dc := s.directory

	switch verb {
	case "dir-publish":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		l := &networkv1directory.Listing{}
		if err := message(1, l); err != nil {
			return "", err
		}
		id, err := dc.Publish(ctx, l, key)
		if err != nil {
			return "", err
		}
		return strconv.FormatUint(id, 10), nil

	case "dir-unpublish", "dir-part":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		id, err := uintArg(1)
		if err != nil {
			return "", err
		}
		if verb == "dir-part" {
			return "", dc.Part(ctx, id, key)
		}
		return "", dc.Unpublish(ctx, id, key)

	case "dir-join":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		q := &networkv1directory.ListingQuery{}
		if err := message(1, q); err != nil {
			return "", err
		}
		id, err := dc.Join(ctx, q, key)
		if err != nil {
			return "", err
		}
		return strconv.FormatUint(id, 10), nil

	case "dir-moderate-listing":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		id, err := uintArg(1)
		if err != nil {
			return "", err
		}
		var m *networkv1directory.ListingModeration
		if err := need(3); err != nil {
			return "", err
		}
		if args[2] != "-" {
			m = &networkv1directory.ListingModeration{}
			if err := message(2, m); err != nil {
				return "", err
			}
		}
		return "", dc.ModerateListing(ctx, id, m, key)

	case "dir-moderate-user":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		peerKey, err := bytesArg(1)
		if err != nil {
			return "", err
		}
		var m *networkv1directory.UserModeration
		if err := need(3); err != nil {
			return "", err
		}
		if args[2] != "-" {
			m = &networkv1directory.UserModeration{}
			if err := message(2, m); err != nil {
				return "", err
			}
		}
		return "", dc.ModerateUser(ctx, peerKey, m, key)

	case "dir-listings":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		nw, err := dao.Networks.Get(s.store, id)
		if err != nil {
			return "", err
		}
		res := &networkv1directory.NetworkListings{
			Network: &networkv1directory.Network{
				Id:   nw.Id,
				Name: dao.CertificateRoot(nw.Certificate).GetSubject(),
				Key:  dao.NetworkKey(nw),
			},
		}
		for _, l := range dc.GetListingsByNetworkID(id) {
			res.Listings = append(res.Listings, &networkv1directory.NetworkListingsItem{
				Id:              l.ID,
				Listing:         l.Listing,
				Snippet:         l.Snippet,
				Moderation:      l.Moderation,
				UserCount:       l.UserCount,
				RecentUserCount: l.RecentUserCount,
			})
		}
		b, err := proto.Marshal(res)
		if err != nil {
			return "", err
		}
		return hexs(string(b)), nil

	case "dir-users":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		var users []string
		for _, u := range dc.GetUsersByNetworkID(id) {
			users = append(users, fmt.Sprintf("%d:%s:%s", u.ID, hexs(u.Alias), hexs(string(u.PeerKey))))
		}
		if len(users) == 0 {
			return "-", nil
		}
		return strings.Join(users, ","), nil

	case "dir-snippet":
		networkKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		snippet := &networkv1directory.ListingSnippet{}
		if err := message(1, snippet); err != nil {
			return "", err
		}
		k, err := dao.GenerateKey()
		if err != nil {
			return "", err
		}
		w, err := ppspp.NewWriter(ppspp.WriterOptions{
			SwarmOptions: ppspp.SwarmOptions{LiveWindow: 1 << 10, DeliveryMode: ppspp.MandatoryDeliveryMode},
			Key:          k,
		})
		if err != nil {
			return "", err
		}
		swarm := w.Swarm()
		s.mu.Lock()
		if s.snippetKeys == nil {
			s.snippetKeys = map[string]*key.Key{}
		}
		s.snippetKeys[string(swarm.ID())] = k
		s.mu.Unlock()
		s.transfer.Publish(s.transfer.Add(swarm, nil), networkKey)
		if err := pushSnippet(dc, swarm.ID(), k, snippet); err != nil {
			return "", err
		}
		return swarm.URI().String(), nil

	case "dir-snippet-update":
		if err := need(2); err != nil {
			return "", err
		}
		uri, err := ppspp.ParseURI(args[0])
		if err != nil {
			return "", err
		}
		snippet := &networkv1directory.ListingSnippet{}
		if err := message(1, snippet); err != nil {
			return "", err
		}
		s.mu.Lock()
		k, ok := s.snippetKeys[string(uri.ID)]
		s.mu.Unlock()
		if !ok {
			return "", errors.New("unknown snippet swarm")
		}
		return "", pushSnippet(dc, uri.ID, k, snippet)

	case "dir-config":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		c := &networkv1directory.ServerConfig{}
		if err := message(1, c); err != nil {
			return "", err
		}
		_, err = dao.Networks.Transform(s.store, id, func(n *networkv1.Network) error {
			if n.ServerConfig == nil {
				return errors.New("not an owned network")
			}
			n.ServerConfig.Directory = c
			return nil
		})
		return "", err
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

// pushSnippet serves snippet for the swarm, signed with k unless it is
// signed already.
func pushSnippet(dc directory.Control, id ppspp.SwarmID, k *key.Key, snippet *networkv1directory.ListingSnippet) error {
	if len(snippet.Signature) == 0 {
		if err := dao.SignMessage(snippet, k); err != nil {
			return err
		}
	}
	dc.PushSnippet(id, snippet)
	return nil
}
