package wgutil

import (
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"strings"

	"github.com/aead/ecdh"
)

// GenerateKey creates a base64 encoded ECDH key pair
func GenerateKey() (private, public string, err error) {
	privateKey, publicKey, err := ecdh.X25519().GenerateKey(rand.Reader)
	if err != nil {
		return "", "", err
	}

	privateBytes := privateKey.([32]byte)
	publicBytes := publicKey.([32]byte)

	private = base64.StdEncoding.EncodeToString(privateBytes[:])
	public = base64.StdEncoding.EncodeToString(publicBytes[:])
	return
}

func PublicFromPrivate(private string) (string, error) {
	key, err := base64.StdEncoding.DecodeString(private)
	if err != nil {
		return "", err
	}

	pub := ecdh.X25519().PublicKey(key).([32]byte)
	return base64.StdEncoding.EncodeToString(pub[:]), nil
}

// InterfaceConfig ...
type InterfaceConfig struct {
	PrivateKey string
	Address    string
	ListenPort uint64
	SaveConfig bool
	Peers      []InterfacePeerConfig
}

func (c *InterfaceConfig) String() string {
	var b strings.Builder

	t := `[Interface]
PrivateKey = %s
Address = %s
ListenPort = %d
SaveConfig = %t`
	b.WriteString(fmt.Sprintf(t, c.PrivateKey, c.Address, c.ListenPort, c.SaveConfig))

	for _, p := range c.Peers {
		b.WriteRune('\n')
		b.WriteString(p.String())
	}

	b.WriteRune('\n')
	return b.String()
}

func (c *InterfaceConfig) Strip() string {
	var b strings.Builder

	t := `[Interface]
PrivateKey = %s
ListenPort = %d`
	b.WriteString(fmt.Sprintf(t, c.PrivateKey, c.ListenPort))

	for _, p := range c.Peers {
		b.WriteRune('\n')
		b.WriteString(p.String())
	}

	b.WriteRune('\n')
	return b.String()
}

// InterfacePeerConfig ...
type InterfacePeerConfig struct {
	PublicKey           string
	AllowedIPs          string
	Endpoint            string
	PersistentKeepalive int
}

func (c *InterfacePeerConfig) String() string {
	t := `[Peer]
PublicKey = %s
AllowedIPs = %s
Endpoint = %s
PersistentKeepalive = %d`
	return fmt.Sprintf(t, c.PublicKey, c.AllowedIPs, c.Endpoint, c.PersistentKeepalive)
}
