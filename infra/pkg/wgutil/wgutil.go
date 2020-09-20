package wgutil

import (
	"crypto"
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

// InterfaceConfig ...
type InterfaceConfig struct {
	PrivateKey string
	Address    string
	ListenPort uint64
	Peers      []InterfacePeerConfig
}

func (c *InterfaceConfig) String() string {
	var b strings.Builder

	t := `[Interface]
PrivateKey = %s
Address = %s
ListenPort = %d`
	b.WriteString(fmt.Sprintf(t, c.PrivateKey, c.Address, c.ListenPort))

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

func publicKeyFromPrivate(priv []byte) crypto.PublicKey {
	return ecdh.X25519().PublicKey(priv)
}
