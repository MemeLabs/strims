package logutil

import (
	"encoding/hex"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// ByteHex ...
func ByteHex(key string, val []byte) zapcore.Field {
	return zap.String(key, hex.EncodeToString(val))
}
