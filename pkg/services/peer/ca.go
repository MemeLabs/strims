package peer

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type caService struct {
	Peer *app.Peer
	App  *app.Control
}

func (s *caService) Renew(ctx context.Context, req *pb.CAPeerRenewRequest) (*pb.CAPeerRenewResponse, error) {
	jsonDump(req)
	return nil, errors.New("not implemented")
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}
