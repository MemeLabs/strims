package frontend

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

func newVideoService(logger *zap.Logger, store *dao.ProfileStore) api.VideoService {
	return &videoService{logger, store}
}

// videoService ...
type videoService struct {
	logger *zap.Logger
	store  *dao.ProfileStore
}

// OpenVideoClient ...
func (s *videoService) OpenClient(ctx context.Context, r *pb.OpenVideoClientRequest) (<-chan *pb.VideoClientEvent, error) {
	session, err := contextSession(ctx)
	if err != nil {
		return nil, err
	}

	s.logger.Debug("start swarm...")

	v, err := NewVideoClient(s.logger, r.SwarmKey)
	if err != nil {
		return nil, err
	}

	id := session.Store(v)

	ch := make(chan *pb.VideoClientEvent, 1)

	ch <- &pb.VideoClientEvent{
		Body: &pb.VideoClientEvent_Open_{
			Open: &pb.VideoClientEvent_Open{
				Id: id,
			},
		},
	}

	if r.EmitData {
		go v.SendEvents(ch)
	}

	return ch, nil
}

// OpenVideoServer ...
func (s *videoService) OpenServer(ctx context.Context, r *pb.OpenVideoServerRequest) (*pb.OpenVideoServerResponse, error) {
	session, err := contextSession(ctx)
	if err != nil {
		return nil, err
	}

	s.logger.Debug("start swarm...")

	v, err := NewVideoServer(s.logger)
	if err != nil {
		return nil, err
	}

	id := session.Store(v)

	return &pb.OpenVideoServerResponse{Id: id}, nil
}

// WriteToVideoServer ...
func (s *videoService) WriteToServer(ctx context.Context, r *pb.WriteToVideoServerRequest) (*pb.WriteToVideoServerResponse, error) {
	session, err := contextSession(ctx)
	if err != nil {
		return nil, err
	}

	tif, _ := session.Values.Load(r.Id)
	t, ok := tif.(*VideoServer)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	if _, err := t.Write(r.Data); err != nil {
		return nil, err
	}
	if r.Flush {
		if err := t.Flush(); err != nil {
			return nil, err
		}
	}

	return &pb.WriteToVideoServerResponse{}, nil
}

// PublishSwarm ...
func (s *videoService) PublishSwarm(ctx context.Context, r *pb.PublishSwarmRequest) (*pb.PublishSwarmResponse, error) {
	session, err := contextSession(ctx)
	if err != nil {
		return nil, err
	}

	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	// TODO: this should return an ErrNetworkNotFound...
	svc, ok := ctl.NetworkServices(r.NetworkKey)
	if !ok {
		return nil, errors.New("unknown network")
	}

	tif, _ := session.Load(r.Id)
	t, ok := tif.(SwarmPublisher)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	if err := t.PublishSwarm(svc); err != nil {
		return nil, err
	}

	return &pb.PublishSwarmResponse{}, nil
}
