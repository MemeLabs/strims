package frontend

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
)

type initProfileFunc func(profile *pb.Profile, store *dao.ProfileStore) error

func newProfileService(ctx context.Context, logger *zap.Logger, store kv.BlobStore, initFunc initProfileFunc) (api.ProfileService, error) {
	metadata, err := dao.NewMetadataStore(store)
	if err != nil {
		return nil, err
	}

	return &profileService{
		logger:   logger,
		metadata: metadata,
		init:     initFunc,
	}, nil
}

// profileService ...
type profileService struct {
	logger   *zap.Logger
	metadata *dao.MetadataStore
	store    *dao.ProfileStore
	init     initProfileFunc
}

// Create ...
func (s *profileService) Create(ctx context.Context, r *pb.CreateProfileRequest) (*pb.CreateProfileResponse, error) {
	profile, store, err := dao.CreateProfile(s.metadata, r.Name, r.Password)
	if err != nil {
		return nil, err
	}

	s.store = store
	if err := s.init(profile, store); err != nil {
		return nil, err
	}

	return &pb.CreateProfileResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}

// Delete ...
func (s *profileService) Delete(ctx context.Context, r *pb.DeleteProfileRequest) (*pb.DeleteProfileResponse, error) {
	// if err := dao.DeleteProfile(s.metadata, s.profile); err != nil {
	// 	return nil, err
	// }

	// if err := s.store.Delete(); err != nil {
	// 	return nil, err
	// }

	// return &pb.DeleteProfileResponse{}, nil
	return &pb.DeleteProfileResponse{}, ErrMethodNotImplemented
}

// Update ...
func (s *profileService) Update(ctx context.Context, r *pb.UpdateProfileRequest) (*pb.UpdateProfileResponse, error) {
	return &pb.UpdateProfileResponse{}, ErrMethodNotImplemented
}

// Load ...
func (s *profileService) Load(ctx context.Context, r *pb.LoadProfileRequest) (*pb.LoadProfileResponse, error) {
	profile, store, err := dao.LoadProfile(s.metadata, r.Id, r.Password)
	if err != nil {
		return nil, err
	}

	s.store = store
	if err := s.init(profile, store); err != nil {
		return nil, err
	}

	return &pb.LoadProfileResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}

// Get ...
func (s *profileService) Get(ctx context.Context, r *pb.GetProfileRequest) (*pb.GetProfileResponse, error) {
	profile, err := dao.GetProfile(s.store)
	if err != nil {
		return nil, err
	}

	return &pb.GetProfileResponse{Profile: profile}, nil
}

// List ...
func (s *profileService) List(ctx context.Context, r *pb.ListProfilesRequest) (*pb.ListProfilesResponse, error) {
	profiles, err := dao.GetProfileSummaries(s.metadata)
	if err != nil {
		return nil, err
	}

	return &pb.ListProfilesResponse{Profiles: profiles}, nil
}

// LoadSession ...
func (s *profileService) LoadSession(ctx context.Context, r *pb.LoadSessionRequest) (*pb.LoadSessionResponse, error) {
	id, storageKey, err := unmarshalSessionID(r.SessionId)
	if err != nil {
		return nil, err
	}

	profile, store, err := dao.LoadProfileFromSession(s.metadata, id, storageKey)
	if err != nil {
		return nil, err
	}

	s.store = store
	if err := s.init(profile, store); err != nil {
		return nil, err
	}

	return &pb.LoadSessionResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}
