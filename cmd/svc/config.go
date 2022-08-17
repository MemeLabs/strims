// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"encoding/base64"
	"fmt"
	"os"
	"time"

	"gopkg.in/yaml.v3"
)

type Optional[T any] struct {
	ok bool
	v  T
}

func (v Optional[T]) String() string {
	if v.ok {
		return fmt.Sprintf("%v", v.v)
	}
	return "<nil>"
}

func (v Optional[T]) Ok() bool {
	return v.ok
}

func (v Optional[T]) Get(fallback T) T {
	if v.ok {
		return v.v
	}
	return fallback
}

func (v Optional[T]) MustGet() T {
	if v.ok {
		return v.v
	}
	panic("config value not set")
}

func (v *Optional[T]) UnmarshalYAML(node *yaml.Node) error {
	v.ok = true
	return node.Decode(&v.v)
}

type Bytes []byte

func (v Bytes) String() string {
	return base64.StdEncoding.EncodeToString(v)
}

func (v *Bytes) UnmarshalYAML(node *yaml.Node) (err error) {
	*v, err = base64.StdEncoding.DecodeString(node.Value)
	return err
}

func loadConfig[T any](path string) (*T, error) {
	var cfg T

	if path == "" {
		return &cfg, nil
	}

	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	if err := yaml.NewDecoder(f).Decode(&cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

type StorageConfig struct {
	Adapter Optional[string] `yaml:"adapter"`
	BBolt   struct {
		Path Optional[string] `yaml:"path"`
	} `yaml:"bbolt"`
	Postgres struct {
		ConnStr       Optional[string] `yaml:"connStr"`
		EnableLogging bool             `yaml:"enableLogging"`
		MaxConns      int32            `yaml:"maxConns"`
	} `yaml:"postgres"`
}

type WebSocketConfig struct {
	WriteTimeout Optional[time.Duration] `yaml:"writeTimeout"`
	ReadTimeout  Optional[time.Duration] `yaml:"readTimeout"`
	PingInterval Optional[time.Duration] `yaml:"pingInterval"`
}

type HTTPConfig struct {
	Address Optional[string] `yaml:"address"`
	TLS     struct {
		Cert Optional[string] `yaml:"cert"`
		Key  Optional[string] `yaml:"key"`
	} `yaml:"tls"`
	WebSocket WebSocketConfig `yaml:"websocket"`
}

type PeerConfig struct {
	Metrics struct {
		Address Optional[string] `yaml:"address"`
	} `yaml:"metrics"`
	Debug struct {
		Address Optional[string] `yaml:"address"`
	}
	Storage StorageConfig `yaml:"storage"`
	Queue   struct {
		Adapter  Optional[string] `yaml:"adapter"`
		Postgres struct {
			ConnStr       Optional[string] `yaml:"connStr"`
			EnableLogging bool             `yaml:"enableLogging"`
			MaxConns      int32            `yaml:"maxConns"`
		} `yaml:"postgres"`
	} `yaml:"queue"`
	HTTP    HTTPConfig `yaml:"http"`
	Session struct {
		Remote struct {
			Enabled Optional[bool] `yaml:"enabled"`
		} `yaml:"remote"`
		Headless []struct {
			ID  uint64 `yaml:"id"`
			Key Bytes  `yaml:"key"`
		} `yaml:"headless"`
	} `yaml:"session"`
	VNIC struct {
		Label  Optional[string] `yaml:"label"`
		WebRTC struct {
			ICEServers        Optional[[]string] `yaml:"iceServers"`
			Enabled           Optional[bool]     `yaml:"enabled"`
			PortMin           uint16             `yaml:"portMin"`
			PortMax           uint16             `yaml:"portMax"`
			UDPMuxAddress     Optional[string]   `yaml:"udpMuxAddress"`
			TCPMuxAddress     Optional[string]   `yaml:"tcpMuxAddress"`
			TCPReadBufferSize Optional[int]      `yaml:"tcpReadBufferSize"`
			EnableLogging     bool               `yaml:"enableLogging"`
		} `yaml:"webrtc"`
		WebSocket struct {
			Enabled Optional[bool] `yaml:"enabled"`
			WebSocketConfig
		} `yaml:"websocket"`
	} `yaml:"vnic"`
}

type InviteServerConfig struct {
	Metrics struct {
		Address Optional[string] `yaml:"address"`
	} `yaml:"metrics"`
	Debug struct {
		Address Optional[string] `yaml:"address"`
	}
	Storage StorageConfig `yaml:"storage"`
	HTTP    HTTPConfig    `yaml:"http"`
}
