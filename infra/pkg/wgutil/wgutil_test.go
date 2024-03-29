// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package wgutil

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"golang.zx2c4.com/wireguard/wgctrl/wgtypes"
)

func TestGenerateKey(t *testing.T) {
	priv, _, err := GenerateKey()
	assert.Nil(t, err)

	_, err = wgtypes.ParseKey(priv)
	assert.Nil(t, err)
}

func TestConfig(t *testing.T) {
	cfg := InterfaceConfig{
		PrivateKey: "QGlZp+MxF1N+nZ4etcXg2tFkxgdCuooJq86v9wJOxko=",
		Address:    "10.0.0.1/32",
		ListenPort: 51820,
		Peers: []InterfacePeerConfig{
			{
				PublicKey:  "oJ+yj94MoLJOsuZyOB+r9u2BrPW+FUiASCqL/+Xpq34=",
				AllowedIPs: "10.0.0.2/32",
				Endpoint:   "node0.strims.gg:51820",
			},
		},
	}

	cstr := `[Interface]
PrivateKey = QGlZp+MxF1N+nZ4etcXg2tFkxgdCuooJq86v9wJOxko=
Address = 10.0.0.1/32
ListenPort = 51820
[Peer]
PublicKey = oJ+yj94MoLJOsuZyOB+r9u2BrPW+FUiASCqL/+Xpq34=
AllowedIPs = 10.0.0.2/32
Endpoint = node0.strims.gg:51820
PersistentKeepalive = 0
`
	assert.Equal(t, cstr, cfg.String())
}
