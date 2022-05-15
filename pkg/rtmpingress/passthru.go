// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package rtmpingress

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"math"
	"net"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/edgeware/mp4ff/aac"
	"github.com/edgeware/mp4ff/avc"
	"github.com/edgeware/mp4ff/mp4"
	"github.com/nareix/joy5/av"
	"github.com/nareix/joy5/format/flv/flvio"
	"github.com/nareix/joy5/format/rtmp"
	"go.uber.org/multierr"
	"go.uber.org/zap"
)

type PassthruServer struct {
	Addr         string
	CheckOrigin  func(a *StreamAddr, c *Conn) bool
	HandleStream func(a *StreamAddr, c *Conn) (ioutil.WriteFlusher, error)
	BaseContext  func(nc net.Conn) context.Context
	Logger       *zap.Logger

	listener net.Listener
	conns    sync.Map
}

func (s *PassthruServer) logEvent(c *rtmp.Conn, nc net.Conn, e int) {
	if s.Logger != nil {
		s.Logger.Debug(
			"rtmp event",
			zap.Stringer("localAddr", nc.LocalAddr()),
			zap.Stringer("remoteAddr", nc.RemoteAddr()),
			zap.String("event", rtmp.EventString[e]),
		)
	}
}

func (s *PassthruServer) handleConn(c *rtmp.Conn, nc net.Conn) {
	if !c.Publishing {
		nc.Close()
		return
	}

	var k string
	if _, err := fmt.Sscanf(c.URL.Path, rtmpPathPattern, &k); err != nil {
		return
	}

	ic := NewConn(s.connContext(nc), nc)

	a := &StreamAddr{
		Key: k,
		URI: fmt.Sprintf(rtmpURIPattern, s.Addr, k),
	}

	if s.CheckOrigin != nil && !s.CheckOrigin(a, ic) {
		ic.Close()
		return
	}

	go func() {
		defer ic.Close()

		w, err := s.HandleStream(a, ic)
		if err != nil {
			s.Logger.Debug("stream handler failed", zap.Error(err))
			return
		}

		err = s.transmux(c, w)
		s.Logger.Debug("transmuxing ended", zap.Error(err))
	}()
}

func (s *PassthruServer) connContext(nc net.Conn) context.Context {
	if s.BaseContext != nil {
		return s.BaseContext(nc)
	}
	return context.Background()
}

func (s *PassthruServer) transmux(c *rtmp.Conn, w ioutil.WriteFlusher) error {
	init := mp4.NewMP4Init()
	init.AddChild(&mp4.FtypBox{
		MajorBrand:       "iso5",
		MinorVersion:     512,
		CompatibleBrands: []string{"iso6", "mp41"},
	})

	moov := mp4.NewMoovBox()
	moov.AddChild(mp4.CreateMvhd())
	moov.AddChild(mp4.NewMvexBox())
	init.AddChild(moov)

	init.AddEmptyTrack(0, "video", "und")
	init.AddEmptyTrack(1, "audio", "und")

	var track [2][]av.Packet
	var timeScale [2]float64
	var baseMediaDecodeTime [2]time.Duration
	var totalSize [2]int

	seq := uint32(1)

	var ib []byte

	for {
		pkt, err := c.ReadPacket()
		if err != nil {
			return fmt.Errorf("reading rtmp packet: %w", err)
		}

		// HAX: chunked stream output gets scuffed by short keyframes so we
		// probably need a limit here but it should at least be configurable
		if pkt.IsKeyFrame && totalSize[0]+totalSize[1] >= 32*1024 {
			if ib == nil {
				f := mp4.NewFile()
				f.AddChild(init.Ftyp, 0)
				f.AddChild(init.Moov, 0)

				var b bytes.Buffer
				if err := f.Encode(&b); err != nil {
					return fmt.Errorf("encoding mp4 init: %w", err)
				}

				ib = make([]byte, b.Len()+2)
				binary.BigEndian.PutUint16(ib, uint16(b.Len()))
				b.Read(ib[2:])
			}

			if _, err := w.Write(ib); err != nil {
				return fmt.Errorf("writing mp4 init: %w", err)
			}

			segmentOffsetTime := baseMediaDecodeTime

			var traf [2]mp4.TrafBox
			for i, pkts := range track {
				traf[i].AddChild(mp4.CreateTfhd(uint32(i + 1)))
				traf[i].AddChild(mp4.CreateTfdt(uint64(math.Round(float64(baseMediaDecodeTime[i]) * timeScale[i]))))
				traf[i].AddChild(mp4.CreateTrun(0))

				for _, pkt := range pkts {
					dur := pkt.Time - baseMediaDecodeTime[i]
					baseMediaDecodeTime[i] = pkt.Time

					traf[i].Trun.AddSample(mp4.Sample{
						Dur:                   uint32(math.Round(float64(dur) * timeScale[i])),
						Size:                  uint32(len(pkt.Data)),
						CompositionTimeOffset: int32(math.Round(float64(pkt.CTime) * timeScale[i])),
					})
				}
			}

			mdat := &mp4.MdatBox{
				Data: make([]byte, totalSize[0]+totalSize[1]),
			}
			var n int
			for _, pkts := range track {
				for _, pkt := range pkts {
					n += copy(mdat.Data[n:], pkt.Data)
				}
			}

			styp := &mp4.StypBox{
				MajorBrand:       "msdh",
				CompatibleBrands: []string{"msdh", "msix"},
			}
			if err := styp.Encode(w); err != nil {
				return fmt.Errorf("encoding stype: %w", err)
			}

			for i := range track {
				if len(track[i]) == 0 {
					continue
				}
				sidx := &mp4.SidxBox{
					Version:                  1,
					ReferenceID:              uint32(i + 1),
					Timescale:                uint32(timeScale[i] * float64(time.Second)),
					EarliestPresentationTime: uint64(math.Round(float64(track[i][0].Time) * timeScale[i])),
					SidxRefs: []mp4.SidxRef{
						{
							ReferencedSize:     uint32(totalSize[0] + totalSize[1]),
							SubSegmentDuration: uint32(math.Round(float64(baseMediaDecodeTime[i]-segmentOffsetTime[i]) * timeScale[0])),
							StartsWithSAP:      1,
						},
					},
				}
				if err := sidx.Encode(w); err != nil {
					return fmt.Errorf("encoding sidx: %w", err)
				}
			}

			moof := &mp4.MoofBox{}
			moof.AddChild(&mp4.MfhdBox{
				SequenceNumber: seq,
			})
			moof.AddChild(&traf[0])
			moof.AddChild(&traf[1])

			traf[0].Trun.DataOffset = int32(moof.Size() + mdat.HeaderSize())
			traf[1].Trun.DataOffset = traf[0].Trun.DataOffset + int32(totalSize[0])

			if err := moof.Encode(w); err != nil {
				return fmt.Errorf("encoding moof: %w", err)
			}
			if err := mdat.Encode(w); err != nil {
				return fmt.Errorf("encoding mdat: %w", err)
			}

			if err := w.Flush(); err != nil {
				return fmt.Errorf("flushing segment: %w", err)
			}

			seq++

			track[0] = track[0][:0]
			track[1] = track[1][:0]
			totalSize[0] = 0
			totalSize[1] = 0
		}

		switch pkt.Type {
		case av.Metadata:
			vals, err := flvio.ParseAMFVals(pkt.Data, false)
			if err != nil {
				return fmt.Errorf("parsing amf header: %w", err)
			}

			if v := vals[0].(flvio.AMFMap).Get("framerate"); v != nil {
				timeScale[0] = (v.V.(float64) * 512.0) / float64(time.Second)
			} else {
				return errors.New("no framerate in amf")
			}
		case av.H264DecoderConfig:
			c, err := avc.DecodeAVCDecConfRec(bytes.NewBuffer(pkt.Data))
			if err != nil {
				return fmt.Errorf("parsing avc decoder config: %w", err)
			}

			init.Moov.Traks[0].SetAVCDescriptor("avc1", c.SPSnalus, c.PPSnalus)
			init.Moov.Traks[0].Mdia.Mdhd.Timescale = uint32(timeScale[0] * float64(time.Second))
			init.Moov.Traks[0].Mdia.Hdlr.Name = "VideoHandler"
			init.Moov.Traks[0].Mdia.Minf.Stbl.Stsd.AvcX.CompressorName = ""
		case av.AACDecoderConfig:
			c, err := aac.DecodeAudioSpecificConfig(bytes.NewBuffer(pkt.Data))
			if err != nil {
				return fmt.Errorf("decoding avc config: %w", err)
			}

			timeScale[1] = float64(c.SamplingFrequency) / float64(time.Second)

			init.Moov.Traks[1].SetAACDescriptor(aac.AAClc, c.SamplingFrequency)
			init.Moov.Traks[1].Mdia.Mdhd.Timescale = uint32(c.SamplingFrequency)
			init.Moov.Traks[1].Mdia.Hdlr.Name = "SoundHandler"
		case av.H264:
			track[0] = append(track[0], pkt)
			totalSize[0] += len(pkt.Data)
		case av.AAC:
			track[1] = append(track[1], pkt)
			totalSize[1] += len(pkt.Data)
		}
	}
}

func (s *PassthruServer) Listen() error {
	listener, err := net.Listen("tcp", s.Addr)
	if err != nil {
		return err
	}
	s.listener = listener

	srv := &rtmp.Server{
		LogEvent:         s.logEvent,
		HandleConn:       s.handleConn,
		HandshakeTimeout: time.Second * 10,
	}

	for {
		nc, err := listener.Accept()
		if err != nil {
			return err
		}

		go func() {
			srv.HandleNetConn(nc)
		}()
	}
}

// Close ...
func (s *PassthruServer) Close() error {
	var errs []error

	if err := s.listener.Close(); err != nil {
		errs = append(errs, err)
	}

	s.conns.Range(func(key, _ any) bool {
		if err := key.(net.Conn).Close(); err != nil {
			errs = append(errs, err)
		}
		return true
	})

	if errs != nil {
		return multierr.Combine(errs...)
	}
	return nil
}
