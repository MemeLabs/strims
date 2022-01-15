package session

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"net/http"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/httputil"
	"github.com/gorilla/websocket"
)

const SessionKeyName = "session_key"
const SessionMaxAge = 86400 * 30

func getOrCreateSessionKey(r *http.Request) ([]byte, error) {
	cookie, err := r.Cookie(SessionKeyName)
	if err == nil {
		key, err := base64.URLEncoding.DecodeString(cookie.Value)
		if err == nil && len(key) == dao.KeySize {
			return key, nil
		}
	}

	key := make([]byte, dao.KeySize)
	if _, err := rand.Read(key); err != nil {
		return nil, err
	}
	return key, nil
}

func createResponseHeader(key []byte) http.Header {
	cookie := &http.Cookie{
		Name:     SessionKeyName,
		Value:    base64.URLEncoding.EncodeToString(key),
		MaxAge:   SessionMaxAge,
		Secure:   true,
		HttpOnly: true,
		SameSite: http.SameSiteNoneMode,
	}
	header := http.Header{}
	header.Add("Set-Cookie", cookie.String())
	return header
}

func KeyHandler(fn func(ctx context.Context, c *websocket.Conn)) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		key, err := getOrCreateSessionKey(r)
		if err != nil {
			http.Error(w, "error initializing session", http.StatusInternalServerError)
			return
		}

		c, err := httputil.DefaultUpgrader.Upgrade(w, r, createResponseHeader(key))
		if err != nil {
			http.Error(w, "error upgrading websocket", http.StatusInternalServerError)
			return
		}

		fn(ContextWithSessionKey(r.Context(), key), c)
	}
}

type sessionKeyKey struct{}

func ContextWithSessionKey(ctx context.Context, sessionKey []byte) context.Context {
	return context.WithValue(ctx, sessionKeyKey{}, sessionKey)
}

func ContextSessionKey(ctx context.Context) []byte {
	if k := ctx.Value(sessionKeyKey{}); k != nil {
		return k.([]byte)
	}
	return nil
}
