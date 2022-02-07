package wgutil

import (
	"crypto/rand"
	"encoding/base64"
	"strconv"
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
	DNS        string
	ListenPort uint64
	SaveConfig bool
	Peers      []InterfacePeerConfig
}

func (c *InterfaceConfig) String() string {
	var b strings.Builder

	b.WriteString("[Interface]")

	b.WriteString("\nPrivateKey = ")
	b.WriteString(c.PrivateKey)

	b.WriteString("\nAddress = ")
	b.WriteString(c.Address)

	if c.DNS != "" {
		b.WriteString("\nDNS = ")
		b.WriteString(c.DNS)
	}
	if c.ListenPort != 0 {
		b.WriteString("\nListenPort = ")
		b.WriteString(strconv.FormatUint(c.ListenPort, 10))
	}
	if c.SaveConfig {
		b.WriteString("\nSaveConfig = ")
		b.WriteString(strconv.FormatBool(c.SaveConfig))
	}

	b.WriteRune('\n')

	for _, p := range c.Peers {
		p.WriteToStringsBuilder(&b)
	}

	return b.String()
}

// InterfacePeerConfig ...
type InterfacePeerConfig struct {
	PublicKey           string
	AllowedIPs          string
	Endpoint            string
	PersistentKeepalive int
	Comment             string
}

func (c *InterfacePeerConfig) String() string {
	var b strings.Builder
	c.WriteToStringsBuilder(&b)
	return b.String()
}

func (c *InterfacePeerConfig) WriteToStringsBuilder(b *strings.Builder) {
	b.WriteString("[Peer]")

	b.WriteString("\nPublicKey = ")
	b.WriteString(c.PublicKey)

	b.WriteString("\nAllowedIPs = ")
	b.WriteString(c.AllowedIPs)

	b.WriteString("\nEndpoint = ")
	b.WriteString(c.Endpoint)

	b.WriteString("\nPersistentKeepalive = ")
	b.WriteString(strconv.Itoa(c.PersistentKeepalive))

	b.WriteRune('\n')
}
