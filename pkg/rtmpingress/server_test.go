package rtmpingress

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net"
	"os"
	"os/exec"
	"path"
	"testing"
	"time"

	"github.com/nareix/joy5/format/rtmp"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

var rtmpServerAddr string = ":9999"

func TestServerTranscodesMultipleStreams(t *testing.T) {
	tcs := []struct {
		tw      *tw
		w, h    int
		variant string
	}{
		{
			newTw(),
			640, 360,
			"source",
		},
		{
			newTw(),
			426, 240,
			"240",
		},
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		t.Fatal(err)
	}

	z := NewTranscoder(logger)
	rtmp := Server{
		Addr: rtmpServerAddr,
		HandleStream: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) {
			for _, tc := range tcs {
				go func(variant string, tw *tw) {
					if err := z.Transcode(a.URI, a.Key, variant, tw); err != nil {
						log.Println(err)
					}
				}(tc.variant, tc.tw)
			}
		},
		CheckOrigin: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) bool {
			return true
		},
	}
	go func() {
		if err := rtmp.Listen(); err != nil {
			log.Println(err)
		}
	}()
	defer rtmp.Close()

	time.Sleep(500 * time.Millisecond)

	err = sendStream(t, path.Join("testdata", "sample.mp4"), fmt.Sprintf("rtmp://%s/live/test1", rtmpServerAddr))
	assert.Nil(t, err, "failed sending stream")

	for _, tc := range tcs {
		files, err := ioutil.ReadDir(tc.tw.path)
		assert.Nil(t, err, fmt.Sprintf("failed to read prob dir %s", tc.tw.path))
		for _, file := range files {
			y := path.Join(tc.tw.path, file.Name())
			data := probeFile(t, y)
			assert.Equal(t, 2, len(data.Streams))
			assert.Equal(t, "h264", data.Streams[0].CodecName)
			assert.Equal(t, "aac", data.Streams[1].CodecName)
			assert.Equal(t, tc.h, data.Streams[0].Height)
			assert.Equal(t, tc.w, data.Streams[0].Width)
		}
	}
}

func TestServerClosesStreamOnCheckOriginReject(t *testing.T) {
	logger, err := zap.NewDevelopment()
	if err != nil {
		t.Fatal(err)
	}

	z := NewTranscoder(logger)
	rtmp := Server{
		Addr: rtmpServerAddr,
		HandleStream: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) {
			go func() {
				if err := z.Transcode(a.URI, a.Key, "source", newTw()); err != nil {
					log.Println(err)
				}
			}()
		},
		CheckOrigin: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) bool {
			return false
		},
	}
	go func() {
		if err := rtmp.Listen(); err != nil {
			log.Println(err)
		}
	}()
	defer rtmp.Close()

	time.Sleep(500 * time.Millisecond)

	err = sendStream(t, path.Join("testdata", "sample.mp4"), fmt.Sprintf("rtmp://%s/live/test1", rtmpServerAddr))
	assert.Error(t, err, "failed to close stream on checkOrigin reject")
}

func TestServerAcceptsMultipleStreams(t *testing.T) {
	handleCalled, checkOriginCalled := false, false
	tsfolders := []string{}

	logger, err := zap.NewDevelopment()
	if err != nil {
		t.Fatal(err)
	}

	x := NewTranscoder(logger)
	rtmp := Server{
		Addr: rtmpServerAddr,
		HandleStream: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) {
			handleCalled = true
			z := newTw()
			go func() {
				if err := x.Transcode(a.URI, a.Key, "source", z); err != nil {
					log.Println(err)
				}
			}()
			tsfolders = append(tsfolders, z.path)
		},
		CheckOrigin: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) bool {
			checkOriginCalled = true
			return true
		},
	}
	go func() {
		if err := rtmp.Listen(); err != nil {
			log.Println(err)
		}
	}()
	defer rtmp.Close()

	time.Sleep(500 * time.Millisecond)

	err = sendStream(t, path.Join("testdata", "sample.mp4"), fmt.Sprintf("rtmp://%s/live/test1", rtmpServerAddr))
	assert.Nil(t, err, "failed sending stream")

	assert.True(t, handleCalled, "HandleStream should be called")
	assert.True(t, checkOriginCalled, "CheckOrigin should be called")

	for _, folder := range tsfolders {
		files, err := ioutil.ReadDir(folder)
		assert.Nil(t, err, fmt.Sprintf("failed to read prob dir %s", folder))
		for _, file := range files {
			data := probeFile(t, path.Join(folder, file.Name()))
			assert.Equal(t, 2, len(data.Streams))
			assert.Equal(t, "h264", data.Streams[0].CodecName)
			assert.Equal(t, "aac", data.Streams[1].CodecName)
			assert.Equal(t, 360, data.Streams[0].Height)
			assert.Equal(t, 640, data.Streams[0].Width)
		}
	}
}

func probeFile(t *testing.T, filename string) *ffprobeResp {
	t.Helper()
	ffprobe, err := exec.LookPath("ffprobe")
	assert.Nil(t, err, "failed to probe file: %s", filename)

	cmd := exec.Command(
		ffprobe, "-loglevel", "fatal",
		"-print_format", "json",
		"-show_format",
		"-show_streams", filename,
	)

	var outb, errb bytes.Buffer
	cmd.Stderr = &errb
	cmd.Stdout = &outb
	if err := cmd.Run(); err != nil {
		t.Fatalf("failed to probe file (%q) %v:", cmd.String(), err)
	}

	var data ffprobeResp
	assert.Nil(t, json.Unmarshal(outb.Bytes(), &data), "failed to unmarshal data: %s", outb.String())
	return &data
}

func sendStream(t *testing.T, samplepath, addr string) error {
	t.Helper()
	_, err := exec.LookPath("ffmpeg")
	assert.Nil(t, err, "ffmpeg is not in $PATH. %v", err)

	cmd := exec.Command(
		"ffmpeg",
		"-re",
		"-i", samplepath,
		"-t", "00:00:05.0",
		"-c", "copy",
		"-f", "flv", addr,
	)

	var errb bytes.Buffer
	cmd.Stderr = &errb
	cmd.Stdout = os.Stdout
	err = cmd.Run()
	if err != nil {
		return fmt.Errorf("cmd failed: %s", errb.String())
	}

	return nil
}

type tw struct {
	i    int
	path string
	file io.WriteCloser
}

func newTw() *tw {
	tmp, err := ioutil.TempDir("", "ppspp")
	if err != nil {
		panic(err)
	}
	return &tw{path: tmp}
}

func (t *tw) Write(p []byte) (int, error) {
	if t.file == nil {
		f, err := os.Create(path.Join(t.path, fmt.Sprintf("%d.mp4", t.i)))
		if err != nil {
			return 0, err
		}
		t.file = f
		t.i++

		// trim moov atom length header
		p = p[2:]
	}

	n, err := t.file.Write(p)
	if err != nil {
		return 0, err
	}

	return n, nil
}

func (t *tw) Flush() error {
	if err := t.file.Close(); err != nil {
		return err
	}

	t.file = nil

	return nil
}

func (t *tw) Close() error {
	log.Println("CLOSE ...")
	return nil
}

type ffprobeResp struct {
	Streams []struct {
		Index              int    `json:"index"`
		CodecName          string `json:"codec_name"`
		CodecLongName      string `json:"codec_long_name"`
		Profile            string `json:"profile"`
		CodecType          string `json:"codec_type"`
		CodecTimeBase      string `json:"codec_time_base"`
		CodecTagString     string `json:"codec_tag_string"`
		CodecTag           string `json:"codec_tag"`
		Width              int    `json:"width,omitempty"`
		Height             int    `json:"height,omitempty"`
		CodedWidth         int    `json:"coded_width,omitempty"`
		CodedHeight        int    `json:"coded_height,omitempty"`
		HasBFrames         int    `json:"has_b_frames,omitempty"`
		SampleAspectRatio  string `json:"sample_aspect_ratio,omitempty"`
		DisplayAspectRatio string `json:"display_aspect_ratio,omitempty"`
		PixFmt             string `json:"pix_fmt,omitempty"`
		Level              int    `json:"level,omitempty"`
		ChromaLocation     string `json:"chroma_location,omitempty"`
		FieldOrder         string `json:"field_order,omitempty"`
		Refs               int    `json:"refs,omitempty"`
		IsAvc              string `json:"is_avc,omitempty"`
		NalLengthSize      string `json:"nal_length_size,omitempty"`
		ID                 string `json:"id"`
		RFrameRate         string `json:"r_frame_rate"`
		AvgFrameRate       string `json:"avg_frame_rate"`
		TimeBase           string `json:"time_base"`
		StartPts           int    `json:"start_pts"`
		StartTime          string `json:"start_time"`
		DurationTs         int    `json:"duration_ts"`
		Duration           string `json:"duration"`
		BitsPerRawSample   string `json:"bits_per_raw_sample,omitempty"`
		Disposition        struct {
			Default         int `json:"default"`
			Dub             int `json:"dub"`
			Original        int `json:"original"`
			Comment         int `json:"comment"`
			Lyrics          int `json:"lyrics"`
			Karaoke         int `json:"karaoke"`
			Forced          int `json:"forced"`
			HearingImpaired int `json:"hearing_impaired"`
			VisualImpaired  int `json:"visual_impaired"`
			CleanEffects    int `json:"clean_effects"`
			AttachedPic     int `json:"attached_pic"`
			TimedThumbnails int `json:"timed_thumbnails"`
		} `json:"disposition"`
		SampleFmt     string `json:"sample_fmt,omitempty"`
		SampleRate    string `json:"sample_rate,omitempty"`
		Channels      int    `json:"channels,omitempty"`
		ChannelLayout string `json:"channel_layout,omitempty"`
		BitsPerSample int    `json:"bits_per_sample,omitempty"`
		BitRate       string `json:"bit_rate,omitempty"`
	} `json:"streams"`
	Format struct {
		Filename       string `json:"filename"`
		NbStreams      int    `json:"nb_streams"`
		NbPrograms     int    `json:"nb_programs"`
		FormatName     string `json:"format_name"`
		FormatLongName string `json:"format_long_name"`
		StartTime      string `json:"start_time"`
		Duration       string `json:"duration"`
		Size           string `json:"size"`
		BitRate        string `json:"bit_rate"`
		ProbeScore     int    `json:"probe_score"`
	} `json:"format"`
}
