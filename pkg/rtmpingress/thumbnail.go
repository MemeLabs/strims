//go:build !js

package rtmpingress

import (
	"bytes"
	"fmt"
	"image"
	"image/jpeg"
	"io"
	"math"

	"github.com/3d0c/gmf"
)

// TODO: gmf leaks like a fucking sieve. we might be better off writing the
// files to disk and generating images with ffmpeg...

func init() {
	gmf.LogSetLevel(gmf.AV_LOG_QUIET)
}

func GetImageFromMp4(src []byte) ([]byte, error) {
	ctx := gmf.NewCtx()
	defer ctx.Free()

	if err := ctx.SetInputFormat("mp4"); err != nil {
		return nil, err
	}

	avioCtx, err := newAVIOContextFromBytes(ctx, src)
	defer avioCtx.Free()
	if err != nil {
		return nil, err
	}

	ctx.SetPb(avioCtx)
	ctx.OpenInput("")

	img, err := getImageFromFmtCtx(ctx)

	for i := 0; i < ctx.StreamsCnt(); i++ {
		st, _ := ctx.GetStream(i)
		st.CodecCtx().Free()
		st.Free()
	}

	return img, err
}

func newAVIOContextFromBytes(ctx *gmf.FmtCtx, b []byte) (*gmf.AVIOContext, error) {
	r := bytes.NewReader(b)
	readPacket := func() ([]byte, int) {
		b := make([]byte, gmf.IO_BUFFER_SIZE)
		n, _ := r.Read(b)
		return b, n
	}

	return gmf.NewAVIOContext(ctx, &gmf.AVIOHandlers{ReadPacket: readPacket})
}

func getImageFromFmtCtx(ctx *gmf.FmtCtx) (img []byte, err error) {
	srcStream, err := ctx.GetBestStream(gmf.AVMEDIA_TYPE_VIDEO)
	if err != nil {
		return nil, fmt.Errorf("no video stream found: %w", err)
	}
	srcCodecCtx := srcStream.CodecCtx()

	width, height := containDimensions(srcCodecCtx.Width(), srcCodecCtx.Height(), 640)

	dstCodec, err := gmf.FindEncoder(gmf.AV_CODEC_ID_RAWVIDEO)
	if err != nil {
		return nil, err
	}
	defer dstCodec.Free()

	dstCtx := gmf.NewCodecCtx(dstCodec)
	defer dstCtx.Free()

	dstCtx.SetTimeBase(gmf.AVR{Num: 1, Den: 1})
	dstCtx.SetPixFmt(gmf.AV_PIX_FMT_RGBA)
	dstCtx.SetWidth(width)
	dstCtx.SetHeight(height)
	if dstCodec.IsExperimental() {
		dstCtx.SetStrictCompliance(gmf.FF_COMPLIANCE_EXPERIMENTAL)
	}

	if err := dstCtx.Open(nil); err != nil {
		return nil, err
	}

	swsCtx, err := gmf.NewSwsCtx(srcCodecCtx.Width(), srcCodecCtx.Height(), srcCodecCtx.PixFmt(), dstCtx.Width(), dstCtx.Height(), dstCtx.PixFmt(), gmf.SWS_BICUBIC)
	defer swsCtx.Free()
	if err != nil {
		return nil, err
	}

	drain := -1
	for drain < 0 && img == nil && err == nil {
		var srcPacket *gmf.Packet
		srcPacket, err = ctx.GetNextPacket()
		if err == io.EOF {
			drain = 0
		} else if err != nil {
			err = fmt.Errorf("reading stream failed: %w", err)
			break
		}

		if srcPacket == nil || srcPacket.StreamIndex() == srcStream.Index() {
			var srcFrames []*gmf.Frame
			srcFrames, err = srcStream.CodecCtx().Decode(srcPacket)
			if err != nil {
				err = fmt.Errorf("video decoding failed: %w", err)
			} else {
				img, err = getImageFromFrames(srcFrames, swsCtx, dstCtx, drain, width, height)
			}

			for _, f := range srcFrames {
				f.Free()
			}
		}

		if srcPacket != nil {
			srcPacket.Free()
		}
	}
	return img, err
}

func getImageFromFrames(srcFrames []*gmf.Frame, swsCtx *gmf.SwsCtx, dstCtx *gmf.CodecCtx, drain int, width, height int) (img []byte, err error) {
	if len(srcFrames) == 0 && drain < 0 {
		return nil, nil
	}

	swsFrames, err := gmf.DefaultRescaler(swsCtx, srcFrames)
	if err != nil {
		return nil, err
	}

	dstPackets, err := dstCtx.Encode(swsFrames, drain)
	if err != nil {
		err = fmt.Errorf("frame encoding failed: %w", err)
	}

	if len(dstPackets) > 0 {
		img, err = encodeImage(dstPackets[0].Data(), width, height)
	}

	for _, p := range dstPackets {
		p.Free()
	}

	for _, f := range swsFrames {
		f.Free()
	}

	return img, err
}

func encodeImage(data []byte, width, height int) ([]byte, error) {
	img := &image.RGBA{}
	img.Pix = data
	img.Stride = 4 * width
	img.Rect = image.Rect(0, 0, width, height)

	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, img, &jpeg.Options{Quality: 75}); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

func containDimensions(width, height, maxSize int) (int, int) {
	if width > maxSize {
		height = int(math.Round(float64(maxSize) / float64(width) * float64(height)))
		width = maxSize
	}
	if height > maxSize {
		width = int(math.Round(float64(maxSize) / float64(height) * float64(width)))
		height = maxSize
	}
	return width, height
}
