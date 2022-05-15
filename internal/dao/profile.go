// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
)

const (
	_ = iota + profileNS
	profileProfileNS
	profileIDNS
)

var Profile = NewSingleton[profilev1.Profile](profileProfileNS, nil)

var profileID = NewSingleton(
	profileIDNS,
	&SingletonOptions[profilev1.ProfileID, *profilev1.ProfileID]{
		DefaultValue: &profilev1.ProfileID{NextId: 1},
	},
)

// NewProfile ...
func NewProfile(name string) (p *profilev1.Profile, err error) {
	p = &profilev1.Profile{
		Name: name,
	}

	p.Key, err = GenerateKey()
	if err != nil {
		return nil, err
	}

	p.Id, err = GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	return p, nil
}
