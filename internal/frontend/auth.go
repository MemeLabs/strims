// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/session"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"go.uber.org/zap"
)

func newAuthService(logger *zap.Logger, store kv.BlobStore, sessionManager *session.Manager, bindServices bindServiceFunc) *authService {
	return &authService{
		logger:         logger,
		store:          store,
		sessionManager: sessionManager,
		bindServices:   bindServices,
	}
}

// authService ...
type authService struct {
	logger         *zap.Logger
	store          kv.BlobStore
	sessionManager *session.Manager
	bindServices   bindServiceFunc
}

func (s *authService) startSessionAndBindServices(profileID uint64, profileKey []byte) (*session.Session, error) {
	session, err := s.sessionManager.GetOrCreateSession(profileID, profileKey)
	if err != nil {
		return nil, err
	}

	s.bindServices(session)
	return session, nil
}

func (s *authService) setClientThingCredentials(u *authv1.LinkedProfile, sessionKey []byte, profileID uint64, profileKey []byte) error {
	if sessionKey != nil {
		token, err := dao.CreateSessionThing(s.store, sessionKey, profileID, profileKey)
		if err != nil {
			return err
		}
		u.Credentials = &authv1.LinkedProfile_Token_{
			Token: &authv1.LinkedProfile_Token{
				ProfileId: profileID,
				Token:     token.Token,
				Eol:       token.EOL,
			},
		}
	} else {
		u.Credentials = &authv1.LinkedProfile_Key_{
			Key: &authv1.LinkedProfile_Key{
				ProfileId:  profileID,
				ProfileKey: profileKey,
			},
		}
	}
	return nil
}

func (s *authService) SignIn(ctx context.Context, req *authv1.SignInRequest) (*authv1.SignInResponse, error) {
	linkedProfile := &authv1.LinkedProfile{}
	var profileID uint64
	var profileKey []byte
	var err error

	sessionKey := session.ContextSessionKey(ctx)
	switch c := req.Credentials.(type) {
	case *authv1.SignInRequest_Password_:
		if c.Password.PairingToken != nil {
			profileID, profileKey, err = dao.OpenServerAuthThing(c.Password.PairingToken.Auth, c.Password.Password)
			if err != nil {
				return nil, err
			}
			if err := dao.ImportPairingToken(s.store, c.Password.PairingToken, profileKey); err != nil {
				return nil, err
			}
		} else {
			profileID, profileKey, err = dao.LoadServerAuthThing(s.store, c.Password.Name, c.Password.Password)
			if err != nil {
				return nil, err
			}
		}

		if c.Password.PersistLogin {
			s.setClientThingCredentials(linkedProfile, sessionKey, profileID, profileKey)
		}
	case *authv1.SignInRequest_Token_:
		sessionToken := &dao.SessionToken{
			Token: c.Token.Token,
			EOL:   c.Token.Eol,
		}
		profileID, profileKey, err = dao.LoadSessionThing(s.store, sessionKey, sessionToken)
		if err != nil {
			return nil, err
		}
	case *authv1.SignInRequest_Key_:
		profileID = c.Key.ProfileId
		profileKey = c.Key.ProfileKey
	}

	session, err := s.startSessionAndBindServices(profileID, profileKey)
	if err != nil {
		return nil, err
	}

	linkedProfile.Name = session.Profile.Name
	return &authv1.SignInResponse{
		LinkedProfile: linkedProfile,
		Profile:       session.Profile,
	}, nil
}

func (s *authService) SignUp(ctx context.Context, req *authv1.SignUpRequest) (*authv1.SignUpResponse, error) {
	profileID, profileKey, err := dao.CreateServerAuthThing(s.store, req.Name, req.Password)
	if err != nil {
		return nil, err
	}

	linkedProfile := &authv1.LinkedProfile{}
	if req.PersistLogin {
		sessionKey := session.ContextSessionKey(ctx)
		s.setClientThingCredentials(linkedProfile, sessionKey, profileID, profileKey)
	}

	session, err := s.startSessionAndBindServices(profileID, profileKey)
	if err != nil {
		return nil, err
	}

	linkedProfile.Name = session.Profile.Name
	return &authv1.SignUpResponse{
		LinkedProfile: linkedProfile,
		Profile:       session.Profile,
	}, nil
}
