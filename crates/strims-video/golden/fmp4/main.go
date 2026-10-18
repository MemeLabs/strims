// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Checks fMP4 records against the Go implementation, for
// crates/strims-video/tests/cross_go.rs.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-video/golden/fmp4 -check <records file>
//	go run -tags golden ./crates/strims-video/golden/fmp4 -passthru <script file> <records file>
//
// A records file holds one record per line, in hex. A record is
// "[u16 BE init length][init segment][media segment]".
//
// -check decodes each record's init and media segments with mp4ff and
// prints, per record:
//
//	record <index> init <len> media <len>
//	ftyp <major brand> <minor version> <compatible brand>,...
//	track <id> <handler> <timescale> <language> <codec> <width>x<height>|<channels>ch <rate>Hz trex <sample description index> <default duration>
//	boxes <top-level box type>,...
//	moof <sequence number>
//	traf <track id> tfdt <base decode time> samples <n> dur <sum> size <sum> sync <n> cto <sum> fnv <hex>
//
// with a traf line per track the fragment holds, in trex order. fnv is the
// FNV-1a 64 hash of the track's sample bytes, found through the data
// offsets; sync counts sync samples by their flags.
//
// -passthru publishes a tag script with a joy5 RTMP client to
// rtmpingress.PassthruServer's transmuxer over loopback TCP and writes the
// records it produces. A script file holds one tag per line,
// "<meta|video|audio> <timestamp ms> <hex body>", with legacy FLV tag
// bodies; a meta body is the AMF0 "onMetaData" script data. A trailing
// AAC packet past the script pushes the client's buffered bytes out; the
// transmuxer writes no record for the segment in progress at the end.
package main

import (
	"bufio"
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"hash/fnv"
	"net"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/rtmpingress"
	"github.com/edgeware/mp4ff/aac"
	"github.com/edgeware/mp4ff/mp4"
	"github.com/nareix/joy5/av"
	"github.com/nareix/joy5/format/flv/flvio"
	"github.com/nareix/joy5/format/rtmp"

	_ "unsafe" // go:linkname
)

//go:linkname transmux github.com/MemeLabs/strims/pkg/rtmpingress.(*PassthruServer).transmux
func transmux(s *rtmpingress.PassthruServer, c *rtmp.Conn, w ioutil.WriteFlusher) error

func main() {
	check := flag.String("check", "", "records file to decode")
	passthru := flag.String("passthru", "", "script file to transmux")
	flag.Parse()
	var err error
	switch {
	case *check != "":
		err = runCheck(*check)
	case *passthru != "" && flag.NArg() == 1:
		err = runPassthru(*passthru, flag.Arg(0))
	default:
		err = errors.New("usage: -check <records> | -passthru <script> <records>")
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func readLines(path string) ([]string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var lines []string
	for _, l := range strings.Split(string(data), "\n") {
		if l = strings.TrimSpace(l); l != "" {
			lines = append(lines, l)
		}
	}
	return lines, nil
}

func runCheck(path string) error {
	lines, err := readLines(path)
	if err != nil {
		return err
	}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	for i, l := range lines {
		record, err := hex.DecodeString(l)
		if err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
		if err := checkRecord(w, i, record); err != nil {
			return fmt.Errorf("record %d: %w", i, err)
		}
	}
	return nil
}

func checkRecord(w *bufio.Writer, index int, record []byte) error {
	if len(record) < 2 {
		return errors.New("no init length")
	}
	n := int(binary.BigEndian.Uint16(record))
	if len(record) < 2+n {
		return errors.New("short init")
	}
	initData, media := record[2:2+n], record[2+n:]
	fmt.Fprintf(w, "record %d init %d media %d\n", index, len(initData), len(media))

	initFile, err := mp4.DecodeFile(bytes.NewReader(initData))
	if err != nil {
		return fmt.Errorf("decoding init: %w", err)
	}
	if initFile.Ftyp == nil || initFile.Moov == nil || initFile.Moov.Mvex == nil {
		return errors.New("init lacks ftyp, moov or mvex")
	}
	ftyp := initFile.Ftyp
	fmt.Fprintf(w, "ftyp %s %d %s\n", ftyp.MajorBrand(), ftyp.MinorVersion(), strings.Join(ftyp.CompatibleBrands(), ","))
	trexs := map[uint32]*mp4.TrexBox{}
	for _, trex := range initFile.Moov.Mvex.Trexs {
		trexs[trex.TrackID] = trex
	}
	for _, trak := range initFile.Moov.Traks {
		line, err := trackLine(trak, trexs)
		if err != nil {
			return err
		}
		fmt.Fprintln(w, line)
	}

	mediaFile, err := mp4.DecodeFile(bytes.NewReader(media))
	if err != nil {
		return fmt.Errorf("decoding media: %w", err)
	}
	var types []string
	for _, b := range mediaFile.Children {
		types = append(types, b.Type())
	}
	fmt.Fprintf(w, "boxes %s\n", strings.Join(types, ","))
	for _, seg := range mediaFile.Segments {
		for _, frag := range seg.Fragments {
			if frag.Moof == nil || frag.Mdat == nil {
				return errors.New("fragment lacks moof or mdat")
			}
			fmt.Fprintf(w, "moof %d\n", frag.Moof.Mfhd.SequenceNumber)
			for _, trex := range initFile.Moov.Mvex.Trexs {
				if err := trafLine(w, frag, trex); err != nil {
					return err
				}
			}
		}
	}
	return nil
}

func trackLine(trak *mp4.TrakBox, trexs map[uint32]*mp4.TrexBox) (string, error) {
	id := trak.Tkhd.TrackID
	mdia := trak.Mdia
	stsd := mdia.Minf.Stbl.Stsd
	var desc string
	switch {
	case stsd.AvcX != nil:
		avcC := stsd.AvcX.AvcC
		if avcC == nil {
			return "", fmt.Errorf("track %d: avc1 lacks avcC", id)
		}
		desc = fmt.Sprintf("%s.%02x%02x%02x %dx%d", stsd.AvcX.Type(), avcC.AVCProfileIndication,
			avcC.ProfileCompatibility, avcC.AVCLevelIndication, stsd.AvcX.Width, stsd.AvcX.Height)
	case stsd.Mp4a != nil:
		esds := stsd.Mp4a.Esds
		if esds == nil {
			return "", fmt.Errorf("track %d: mp4a lacks esds", id)
		}
		asc, err := aac.DecodeAudioSpecificConfig(bytes.NewReader(esds.DecConfigDescriptor.DecSpecificInfo.DecConfig))
		if err != nil {
			return "", fmt.Errorf("track %d: %w", id, err)
		}
		desc = fmt.Sprintf("mp4a.%02x.%d %dch %dHz", esds.DecConfigDescriptor.ObjectType, asc.ObjectType,
			stsd.Mp4a.ChannelCount, stsd.Mp4a.SampleRate)
	default:
		return "", fmt.Errorf("track %d: no avc1 or mp4a sample entry", id)
	}
	trex, ok := trexs[id]
	if !ok {
		return "", fmt.Errorf("track %d has no trex", id)
	}
	return fmt.Sprintf("track %d %s %d %s %s trex %d %d", id, mdia.Hdlr.HandlerType, mdia.Mdhd.Timescale,
		mdia.Mdhd.GetLanguage(), desc, trex.DefaultSampleDescriptionIndex, trex.DefaultSampleDuration), nil
}

func trafLine(w *bufio.Writer, frag *mp4.Fragment, trex *mp4.TrexBox) error {
	var traf *mp4.TrafBox
	for _, t := range frag.Moof.Trafs {
		if t.Tfhd.TrackID == trex.TrackID {
			traf = t
		}
	}
	if traf == nil {
		return nil
	}
	samples, err := frag.GetFullSamples(trex)
	if err != nil {
		return fmt.Errorf("track %d samples: %w", trex.TrackID, err)
	}
	var dur, size uint64
	var sync int
	var cto int64
	h := fnv.New64a()
	for _, s := range samples {
		dur += uint64(s.Dur)
		size += uint64(s.Size)
		cto += int64(s.CompositionTimeOffset)
		if s.IsSync() {
			sync++
		}
		h.Write(s.Data)
	}
	fmt.Fprintf(w, "traf %d tfdt %d samples %d dur %d size %d sync %d cto %d fnv %016x\n", trex.TrackID,
		traf.Tfdt.BaseMediaDecodeTime(), len(samples), dur, size, sync, cto, h.Sum64())
	return nil
}

// recorder collects the records a transmuxer writes: Flush ends one.
type recorder struct {
	current []byte
	records [][]byte
}

func (r *recorder) Write(p []byte) (int, error) {
	r.current = append(r.current, p...)
	return len(p), nil
}

func (r *recorder) Flush() error {
	r.records = append(r.records, r.current)
	r.current = nil
	return nil
}

func scriptPackets(path string) ([]av.Packet, error) {
	lines, err := readLines(path)
	if err != nil {
		return nil, err
	}
	var pkts []av.Packet
	for i, l := range lines {
		f := strings.Fields(l)
		if len(f) != 3 {
			return nil, fmt.Errorf("tag %d: want 3 fields", i)
		}
		ms, err := strconv.ParseUint(f[1], 10, 32)
		if err != nil {
			return nil, fmt.Errorf("tag %d: %w", i, err)
		}
		body, err := hex.DecodeString(f[2])
		if err != nil {
			return nil, fmt.Errorf("tag %d: %w", i, err)
		}
		at := time.Duration(ms) * time.Millisecond
		pkt, err := tagPacket(f[0], at, body)
		if err != nil {
			return nil, fmt.Errorf("tag %d: %w", i, err)
		}
		pkts = append(pkts, pkt)
	}
	return pkts, nil
}

func tagPacket(kind string, at time.Duration, body []byte) (av.Packet, error) {
	switch kind {
	case "meta":
		vals, err := flvio.ParseAMFVals(body, false)
		if err != nil {
			return av.Packet{}, err
		}
		if len(vals) < 2 || vals[0] != "onMetaData" {
			return av.Packet{}, errors.New("meta is not onMetaData")
		}
		return av.Packet{Type: av.Metadata, Data: flvio.FillAMF0ValsMalloc(vals[1:]), Time: at}, nil
	case "video":
		if len(body) < 5 || body[0]&0x0f != flvio.VIDEO_H264 {
			return av.Packet{}, errors.New("video is not legacy AVC")
		}
		cts := int32(uint32(body[2])<<24|uint32(body[3])<<16|uint32(body[4])<<8) >> 8
		switch body[1] {
		case flvio.AVC_SEQHDR:
			return av.Packet{Type: av.H264DecoderConfig, Data: body[5:], Time: at}, nil
		case flvio.AVC_NALU:
			return av.Packet{
				Type:       av.H264,
				IsKeyFrame: body[0]>>4 == flvio.FRAME_KEY,
				Time:       at,
				CTime:      time.Duration(cts) * time.Millisecond,
				Data:       body[5:],
			}, nil
		}
	case "audio":
		if len(body) < 2 || body[0]>>4 != flvio.SOUND_AAC {
			return av.Packet{}, errors.New("audio is not AAC")
		}
		switch body[1] {
		case flvio.AAC_SEQHDR:
			return av.Packet{Type: av.AACDecoderConfig, Data: body[2:], Time: at}, nil
		case flvio.AAC_RAW:
			return av.Packet{Type: av.AAC, Data: body[2:], Time: at}, nil
		}
	}
	return av.Packet{}, fmt.Errorf("unsupported %s tag", kind)
}

func runPassthru(scriptPath, outPath string) error {
	pkts, err := scriptPackets(scriptPath)
	if err != nil {
		return err
	}
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	defer ln.Close()

	rec := &recorder{}
	done := make(chan error, 1)
	go func() {
		nc, err := ln.Accept()
		if err != nil {
			done <- err
			return
		}
		defer nc.Close()
		srv := &rtmp.Server{
			HandshakeTimeout: 10 * time.Second,
			HandleConn: func(c *rtmp.Conn, nc net.Conn) {
				done <- transmux(&rtmpingress.PassthruServer{}, c, rec)
			},
		}
		srv.HandleNetConn(nc)
	}()

	c, nc, err := rtmp.NewClient().Dial("rtmp://"+ln.Addr().String()+"/live/key", rtmp.PrepareWriting)
	if err != nil {
		return fmt.Errorf("dialing: %w", err)
	}
	for i, pkt := range pkts {
		if err := c.WritePacket(pkt); err != nil {
			return fmt.Errorf("writing tag %d: %w", i, err)
		}
	}
	last := pkts[len(pkts)-1].Time
	if err := c.WritePacket(av.Packet{Type: av.AAC, Data: make([]byte, 16*1024), Time: last}); err != nil {
		return fmt.Errorf("writing the trailing packet: %w", err)
	}
	nc.Close()

	select {
	case <-done:
	case <-time.After(30 * time.Second):
		return errors.New("transmux did not end")
	}
	var out strings.Builder
	for _, r := range rec.records {
		out.WriteString(hex.EncodeToString(r))
		out.WriteByte('\n')
	}
	return os.WriteFile(outPath, []byte(out.String()), 0o644)
}
