// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package timeutil

import (
	"encoding/binary"
	"errors"
	"math"
	"time"
)

const NilTime = Time(math.MinInt64)
const MaxTime = Time(math.MaxInt64)
const EpochTime = Time(0)

const TimeLength = 8

const Precision = time.Millisecond

func Since(start Time) time.Duration {
	return time.Duration(Now() - start)
}

func New(epoch int64) Time {
	return Time(epoch)
}

func Unix(sec, nsec int64) Time {
	return Time(sec*int64(time.Second) + nsec)
}

func NewFromTime(t time.Time) Time {
	return Time(t.UnixNano())
}

type Time int64

func (t Time) Add(o time.Duration) Time {
	return t + Time(o)
}

func (t Time) Sub(o Time) time.Duration {
	return time.Duration(t - o)
}

func (t Time) Truncate(d time.Duration) Time {
	return t - (t % Time(d))
}

func (t Time) IsNil() bool {
	return t == NilTime
}

func (t Time) After(o Time) bool {
	return t > o
}

func (t Time) Before(o Time) bool {
	return t < o
}

func (t Time) Unix() int64 {
	return int64(t) / int64(time.Second)
}

func (t Time) UnixNano() int64 {
	return int64(t)
}

func (t Time) Time() time.Time {
	return time.Unix(0, int64(t))
}

func (t Time) String() string {
	return t.Time().String()
}

func (t *Time) Unmarshal(b []byte) (int, error) {
	if len(b) < TimeLength {
		return 0, errors.New("buffer too short")
	}

	*t = Time(binary.BigEndian.Uint64(b))
	return TimeLength, nil
}

func (t Time) Marshal(b []byte) (int, error) {
	if len(b) < TimeLength {
		return 0, errors.New("buffer too short")
	}

	binary.BigEndian.PutUint64(b, uint64(t))
	return 8, nil
}

func UnmarshalTime(b []byte) (t Time, err error) {
	_, err = t.Unmarshal(b)
	return
}
