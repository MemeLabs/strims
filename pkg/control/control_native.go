//go:build !js

package control

// VideoEgressControl ...
type VideoEgressControl interface {
	VideoEgressControlBase
	VideoHLSEgressControl
}
