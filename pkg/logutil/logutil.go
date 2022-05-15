package logutil

import (
	"encoding/hex"
	"reflect"

	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// ByteHex ...
func ByteHex(key string, val []byte) zapcore.Field {
	return zap.String(key, hex.EncodeToString(val))
}

// Type constructs a field with the given key and the name of the type of v
func Type(key string, v any) zapcore.Field {
	return zap.String(key, reflect.TypeOf(v).String())
}

func CheckWithTimer(logger *zap.Logger, lvl zapcore.Level, msg string) (*zapcore.CheckedEntry, *debug.Timer) {
	ce := logger.WithOptions(zap.AddCallerSkip(1)).Check(lvl, msg)
	if ce != nil {
		return ce, debug.StartTimer()
	}
	return ce, nil
}
