// +build !js

package videoegress

// OpenHLSStream ...
func (t *Control) OpenHLSStream(swarmURI string) (string, error) {
	return "", nil
}

// CloseHLSStream ...
func (t *Control) CloseHLSStream(swarmURI string) error {
	return nil
}
