package frontend

import (
	"context"
	"errors"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
)

type initProfileFunc func(profile *profilev1.Profile, store *dao.ProfileStore) error

func newProfileService(logger *zap.Logger, store kv.BlobStore, initFunc initProfileFunc) (profilev1.ProfileServiceService, error) {
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
func (s *profileService) Create(ctx context.Context, r *profilev1.CreateProfileRequest) (*profilev1.CreateProfileResponse, error) {
	profile, store, err := dao.CreateProfile(s.metadata, r.Name, r.Password)
	if err != nil {
		return nil, err
	}

	s.store = store
	if err := s.init(profile, store); err != nil {
		return nil, err
	}

	return &profilev1.CreateProfileResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}

// Delete ...
func (s *profileService) Delete(ctx context.Context, r *profilev1.DeleteProfileRequest) (*profilev1.DeleteProfileResponse, error) {
	// if err := dao.DeleteProfile(s.metadata, s.profile); err != nil {
	// 	return nil, err
	// }

	// if err := s.store.Delete(); err != nil {
	// 	return nil, err
	// }

	// return &profilev1.DeleteProfileResponse{}, nil
	return &profilev1.DeleteProfileResponse{}, rpc.ErrNotImplemented
}

// Update ...
func (s *profileService) Update(ctx context.Context, r *profilev1.UpdateProfileRequest) (*profilev1.UpdateProfileResponse, error) {
	return &profilev1.UpdateProfileResponse{}, rpc.ErrNotImplemented
}

// Load ...
func (s *profileService) Load(ctx context.Context, r *profilev1.LoadProfileRequest) (*profilev1.LoadProfileResponse, error) {
	profile, store, err := dao.LoadProfile(s.metadata, r.Id, r.Password)
	if err != nil {
		return nil, err
	}

	s.store = store
	if err := s.init(profile, store); err != nil {
		return nil, err
	}

	return &profilev1.LoadProfileResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}

// Get ...
func (s *profileService) Get(ctx context.Context, r *profilev1.GetProfileRequest) (*profilev1.GetProfileResponse, error) {
	profile, err := dao.GetProfile(s.store)
	if err != nil {
		return nil, err
	}

	return &profilev1.GetProfileResponse{Profile: profile}, nil
}

// List ...
func (s *profileService) List(ctx context.Context, r *profilev1.ListProfilesRequest) (*profilev1.ListProfilesResponse, error) {
	profiles, err := dao.GetProfileSummaries(s.metadata)
	if err != nil {
		return nil, err
	}

	return &profilev1.ListProfilesResponse{Profiles: profiles}, nil
}

// LoadSession ...
func (s *profileService) LoadSession(ctx context.Context, r *profilev1.LoadSessionRequest) (*profilev1.LoadSessionResponse, error) {
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

	return &profilev1.LoadSessionResponse{
		SessionId: marshalSessionID(profile, store),
		Profile:   profile,
	}, nil
}
