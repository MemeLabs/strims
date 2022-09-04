// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"errors"
	"reflect"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"go.uber.org/multierr"
)

var linkCandidateTypes map[string]reflect.Type

func init() {
	linkCandidateTypes = map[string]reflect.Type{}
}

func RegisterLinkInterface(name string, v LinkCandidate) {
	linkCandidateTypes[name] = reflect.TypeOf(v)
}

type LinkCandidatePool struct {
	candidates []LinkCandidate
}

func (p *LinkCandidatePool) LocalDescriptions() ([]*vnicv1.LinkDescription, error) {
	var ds []*vnicv1.LinkDescription
	var errs []error
	for _, c := range p.candidates {
		d, err := c.LocalDescription()
		if err != nil {
			errs = append(errs, err)
		} else if d != nil {
			ds = append(ds, d)
		}
	}
	if len(ds) == 0 {
		return nil, multierr.Combine(errs...)
	}
	return ds, nil
}

func (p *LinkCandidatePool) SetRemoteDescriptions(ds []*vnicv1.LinkDescription) (bool, error) {
	cs := map[reflect.Type][]LinkCandidate{}
	for _, c := range p.candidates {
		t := reflect.TypeOf(c)
		cs[t] = append(cs[t], c)
	}

	var errs []error
	for _, d := range ds {
		t := linkCandidateTypes[d.Interface]
		for _, c := range cs[t] {
			connected, err := c.SetRemoteDescription(d)

			var peerInitErr *PeerInitError
			if errors.As(err, &peerInitErr) {
				return false, peerInitErr
			} else if err != nil {
				errs = append(errs, err)
			} else if connected {
				return connected, nil
			}
		}
	}
	if len(errs) != 0 {
		return false, multierr.Combine(errs...)
	}
	return false, nil
}
