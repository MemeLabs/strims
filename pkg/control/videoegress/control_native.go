//go:build !js
// +build !js

package videoegress

// OpenHLSStream ...
func (t *Control) OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error) {
	return "", nil
}

// CloseHLSStream ...
func (t *Control) CloseHLSStream(swarmURI string) error {
	return nil
}
