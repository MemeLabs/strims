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
	"sync"
	"testing"
	"time"

	"github.com/nareix/joy5/format/rtmp"
	"github.com/tj/assert"
)

func TestServerAcceptsMultipleStreams(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping test in short mode.")
	}

	handleCalled, checkOriginCalled := false, false
	rtmpServerAddr := ":9999"

	tsfolders := []string{}

	x := Transcoder{}
	rtmp := Server{
		Addr: rtmpServerAddr,
		HandleStream: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) {
			handleCalled = true
			z := newTw()
			go x.Transcode(a.URI, a.Key, "source", z)
			tsfolders = append(tsfolders, z.path)
		},
		CheckOrigin: func(a *StreamAddr, c *rtmp.Conn, nc net.Conn) bool {
			checkOriginCalled = true
			return true
		},
	}
	go rtmp.Listen()

	time.Sleep(500 * time.Millisecond)
	var wg sync.WaitGroup

	send := func(file, url string) {
		if err := sendStream(t, file, url); err != nil {
			panic(err)
		}
		wg.Done()
	}

	wg.Add(2)
	go send(path.Join("testdata", "sample.mp4"), fmt.Sprintf("rtmp://%s/live/test1", rtmpServerAddr))
	go send(path.Join("testdata", "sample.mp4"), fmt.Sprintf("rtmp://%s/live/test2", rtmpServerAddr))
	wg.Wait()

	if !handleCalled || !checkOriginCalled {
		t.Fatal("failed to call handle or checkorigin")
	}

	for _, folder := range tsfolders {
		files, err := ioutil.ReadDir(folder)
		if err != nil {
			t.Error(err)
		}
		for _, file := range files {
			if err := probeFile(t, path.Join(folder, file.Name())); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func probeFile(t *testing.T, filename string) error {
	t.Helper()
	_, err := exec.LookPath("ffprobe")
	if err != nil {
		t.Fatalf("ffprobe is not in $PATH. %v", err)
	}

	cmd := exec.Command(
		"ffprobe", "-loglevel", "fatal",
		"-print_format", "json",
		"-show_format",
		"-show_streams", filename,
	)

	var outb, errb bytes.Buffer
	cmd.Stderr = &errb
	cmd.Stdout = &outb
	if err = cmd.Run(); err != nil {
		return fmt.Errorf("cmd failed: %s", errb.String())
	}

	var data ffprobeResp
	if err := json.Unmarshal(outb.Bytes(), &data); err != nil {
		return fmt.Errorf("failed to unmarshal ffprobe results: %v", err)
	}

	assert.Equal(t, len(data.Streams), 2)
	assert.Equal(t, data.Streams[0].CodecName, "h264")
	assert.Equal(t, data.Streams[1].CodecName, "aac")
	assert.Equal(t, data.Streams[0].Height, 360)
	assert.Equal(t, data.Streams[0].Width, 640)
	return nil
}

func sendStream(t *testing.T, samplepath, addr string) error {
	t.Helper()
	_, err := exec.LookPath("ffmpeg")
	if err != nil {
		t.Fatalf("ffmpeg is not in $PATH. %v", err)
	}

	cmd := exec.Command(
		"ffmpeg",
		"-re",
		"-i", samplepath,
		"-c", "copy",
		"-f", "flv", addr,
	)

	var errb bytes.Buffer
	cmd.Stderr = &errb
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
	f, err := os.Create(path.Join(tmp, "0.ts"))
	if err != nil {
		panic(err)
	}
	return &tw{0, tmp, f}
}

func (t *tw) Write(p []byte) (int, error) {
	log.Println(len(p))

	n, err := t.file.Write(p)
	if err != nil {
		return 0, err
	}

	return n, nil
}

func (t *tw) Flush() error {
	log.Println("FLUSH ...")
	if err := t.file.Close(); err != nil {
		return err
	}

	t.i++
	f, err := os.Create(path.Join(t.path, fmt.Sprintf("%d.ts", t.i)))
	if err != nil {
		return err
	}

	t.file = f

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
