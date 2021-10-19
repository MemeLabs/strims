package rtmpingress

import (
	"fmt"
	"os"
	"os/exec"
	"strconv"

	"go.uber.org/multierr"
)

var (
	DefaultThumbnailerMaxWidth  = 640
	DefaultThumbnailerMaxHeight = 360
	DefaultThumbnailerQuality   = 5
)

type Thumbnailer struct {
	MaxWidth  int
	MaxHeight int
	Quality   int

	src *os.File
	dst string
}

func (t *Thumbnailer) Close() error {
	var errs []error
	if err := t.src.Close(); err != nil {
		errs = append(errs, err)
	}
	if err := os.Remove(t.dst); err != nil {
		errs = append(errs, err)
	}
	if errs != nil {
		return multierr.Combine(errs...)
	}
	return nil
}

func (t *Thumbnailer) init() error {
	if t.src == nil {
		src, err := os.CreateTemp("", "src.*.mp4")
		if err != nil {
			return err
		}
		t.src = src
	}

	if t.dst == "" {
		dst, err := os.CreateTemp("", "dst.*.jpg")
		if err != nil {
			return err
		}
		if err := dst.Close(); err != nil {
			return err
		}
		t.dst = dst.Name()
	}

	if t.MaxWidth == 0 {
		t.MaxWidth = DefaultThumbnailerMaxWidth
	}
	if t.MaxHeight == 0 {
		t.MaxHeight = DefaultThumbnailerMaxHeight
	}
	if t.Quality == 0 {
		t.Quality = DefaultThumbnailerQuality
	}

	return nil
}

func (t *Thumbnailer) GetImageFromMp4(src []byte) ([]byte, error) {
	if err := t.init(); err != nil {
		return nil, err
	}

	if err := t.src.Truncate(int64(len(src))); err != nil {
		return nil, err
	}
	if _, err := t.src.WriteAt(src, 0); err != nil {
		return nil, err
	}

	cmd := exec.Command(
		"ffmpeg",
		"-y",
		"-i", t.src.Name(),
		"-vframes", "1",
		"-vf", fmt.Sprintf(
			"scale=w=%d:h=%d:force_original_aspect_ratio=decrease",
			t.MaxWidth,
			t.MaxHeight,
		),
		"-f", "singlejpeg",
		"-q:v", strconv.Itoa(t.Quality),
		t.dst,
	)
	if err := cmd.Run(); err != nil {
		return nil, err
	}

	return os.ReadFile(t.dst)
}
