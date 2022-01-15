package frontend

import (
	"context"
	"io"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/session"
	authv1 "github.com/MemeLabs/go-ppspp/pkg/apis/auth/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

// use a higher limit here to prevent errors while streaming video from the ui.
const serverMaxMessageBytes = 10 * 1024 * 1024

// Server ...
type Server struct {
	Store          kv.BlobStore
	Logger         *zap.Logger
	SessionManager *session.Manager
}

// Listen ...
func (s *Server) Listen(ctx context.Context, rw io.ReadWriter) error {
	server := rpc.NewServer(s.Logger, &rpc.RWDialer{
		Logger:          s.Logger,
		ReadWriter:      rw,
		MaxMessageBytes: serverMaxMessageBytes,
	})

	authService := newAuthService(s.Logger, s.Store, s.SessionManager, serviceBinder(s.Logger, server))
	authv1.RegisterAuthFrontendService(server, authService)

	return server.Listen(ctx)
}

// ServiceParams ...
type ServiceParams struct {
	Logger  *zap.Logger
	Profile *profilev1.Profile
	Store   *dao.ProfileStore
	App     app.Control
}

// ServiceFunc ...
type ServiceFunc func(server *rpc.Server, params ServiceParams)

var services []ServiceFunc

// RegisterService ...
func RegisterService(fn ServiceFunc) {
	services = append(services, fn)
}

type bindServiceFunc func(session *session.Session)

func serviceBinder(
	logger *zap.Logger,
	server *rpc.Server,
) bindServiceFunc {
	return func(session *session.Session) {
		for _, fn := range services {
			fn(server, ServiceParams{
				Logger:  logger,
				Profile: session.Profile,
				Store:   session.Store,
				App:     session.App,
			})
		}
	}
}
