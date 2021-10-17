//go:build !js
// +build !js

package videoegress

// Control ...
type Control interface {
	ControlBase
	OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error)
	CloseHLSStream(swarmURI string) error
}

// OpenHLSStream ...
func (t *control) OpenHLSStream(swarmURI string, networkKeys [][]byte) (string, error) {
	return "", nil
}

// CloseHLSStream ...
func (t *control) CloseHLSStream(swarmURI string) error {
	return nil
}
