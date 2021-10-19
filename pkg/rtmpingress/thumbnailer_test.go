package rtmpingress

import (
	"bytes"
	"image/jpeg"
	"io/ioutil"
	"path"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestThumbnailer(t *testing.T) {
	var thumbnailer Thumbnailer
	defer thumbnailer.Close()

	b, err := ioutil.ReadFile(path.Join("testdata", "sample.mp4"))
	assert.NoError(t, err)

	img, err := thumbnailer.GetImageFromMp4(b)
	assert.NoError(t, err)

	_, err = jpeg.Decode(bytes.NewReader(img))
	assert.NoError(t, err, "expected valid jpeg image")
}
