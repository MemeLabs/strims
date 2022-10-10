// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package logutil

import (
	"testing"

	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"go.uber.org/zap"
)

func TestProto(t *testing.T) {
	zap.NewNop().Debug("test", Proto("test", &networkv1.ListNetworksResponse{
		Networks: []*networkv1.Network{
			{
				Id: 123,
				Version: &daov1.VersionVector{
					Value: map[uint64]uint64{
						111: 111,
						222: 222,
						333: 333,
					},
					UpdatedAt: 1234,
				},
				Certificate: &certificate.Certificate{
					Key:          make([]byte, 32),
					KeyType:      key.KeyType_KEY_TYPE_ED25519,
					KeyUsage:     certificate.KeyUsage_KEY_USAGE_PEER,
					Subject:      "test",
					NotBefore:    1234,
					NotAfter:     1234,
					SerialNumber: make([]byte, 32),
					Signature:    make([]byte, 32),
					ParentOneof: &certificate.Certificate_Parent{
						Parent: &certificate.Certificate{
							Subject: "test",
						},
					},
				},
				Alias: "test",
				ServerConfig: &networkv1.ServerConfig{
					Name: "test",
					Key: &key.Key{
						Type:    key.KeyType_KEY_TYPE_ED25519,
						Private: make([]byte, 64),
						Public:  make([]byte, 32),
					},
					RootCertTtlSecs: 123,
					PeerCertTtlSecs: 123,
					Directory: &networkv1directory.ServerConfig{
						Integrations: &networkv1directory.ServerConfig_Integrations{
							Angelthump: &networkv1directory.ServerConfig_Integrations_AngelThump{
								Enable: true,
							},
							Twitch: &networkv1directory.ServerConfig_Integrations_Twitch{
								Enable:       true,
								ClientId:     "test",
								ClientSecret: "test",
							},
							Youtube: &networkv1directory.ServerConfig_Integrations_YouTube{
								Enable:       true,
								PublicApiKey: "test",
							},
							Swarm: &networkv1directory.ServerConfig_Integrations_Swarm{
								Enable: true,
							},
						},
						PublishQuota:          123,
						JoinQuota:             123,
						BroadcastInterval:     123,
						RefreshInterval:       123,
						SessionTimeout:        123,
						MinPingInterval:       123,
						MaxPingInterval:       123,
						EmbedLoadInterval:     123,
						LoadMediaEmbedTimeout: 123,
					},
					Icon: &image.Image{
						Type:   image.ImageType_IMAGE_TYPE_PNG,
						Height: 512,
						Width:  512,
						Data:   make([]byte, 3477598),
					},
				},
			},
		},
	}))
}
