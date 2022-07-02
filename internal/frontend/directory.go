// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"
	"errors"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/chanutil"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"go.uber.org/multierr"
	"golang.org/x/exp/slices"
	"golang.org/x/sync/errgroup"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		networkv1directory.RegisterDirectoryFrontendService(server, &directoryService{
			app:   params.App,
			store: params.Store,
		})
	})
}

// directoryService ...
type directoryService struct {
	app   app.Control
	store *dao.ProfileStore
}

func (s *directoryService) client(ctx context.Context, networkKey []byte) (*networkv1directory.DirectoryClient, error) {
	client, err := s.app.Network().Dialer().Client(ctx, networkKey, networkKey, directory.AddressSalt)
	if err != nil {
		return nil, err
	}
	return networkv1directory.NewDirectoryClient(client), nil
}

// Publish ...
func (s *directoryService) Publish(ctx context.Context, r *networkv1directory.FrontendPublishRequest) (*networkv1directory.FrontendPublishResponse, error) {
	id, err := s.app.Directory().Publish(ctx, r.Listing, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendPublishResponse{Id: id}, nil
}

// Unpublish ...
func (s *directoryService) Unpublish(ctx context.Context, r *networkv1directory.FrontendUnpublishRequest) (*networkv1directory.FrontendUnpublishResponse, error) {
	err := s.app.Directory().Unpublish(ctx, r.Id, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendUnpublishResponse{}, nil
}

// Join ...
func (s *directoryService) Join(ctx context.Context, r *networkv1directory.FrontendJoinRequest) (*networkv1directory.FrontendJoinResponse, error) {
	id, err := s.app.Directory().Join(ctx, r.Query, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendJoinResponse{Id: id}, nil
}

// Part ...
func (s *directoryService) Part(ctx context.Context, r *networkv1directory.FrontendPartRequest) (*networkv1directory.FrontendPartResponse, error) {
	err := s.app.Directory().Part(ctx, r.Id, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendPartResponse{}, nil
}

// Test ...
func (s *directoryService) Test(ctx context.Context, r *networkv1directory.FrontendTestRequest) (*networkv1directory.FrontendTestResponse, error) {
	client, err := s.app.Network().Dialer().Client(ctx, r.NetworkKey, r.NetworkKey, directory.AddressSalt)
	if err != nil {
		return nil, err
	}
	directoryClient := networkv1directory.NewDirectoryClient(client)

	sets := []struct {
		service networkv1directory.Listing_Embed_Service
		ids     []string
	}{
		{
			networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
			[]string{"DDTGlyJVNVI", "SocSlBubzwA", "-IO6fpjDJY8", "nKrznsPB5t8", "PMesD2l6viA", "GTYJd2qfx5g", "i6zaVYWLTkU", "9VE7afYWzYo", "JQ4Jx8XfP_w", "oEgZeTr3Vs4", "pRpeEdMmmQ0"},
		},
		{
			networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
			[]string{"psrngafk", "bliutwo", "t4tv", "somuchforsubtlety", "erik", "windowsmoviehouse", "keyno", "eastmancolor", "feenamabob", "harkdan"},
		},
		{
			networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM,
			[]string{"namemannen", "not0like0this", "buddha", "shroud", "purgegamers", "sweetdreams", "destiny", "hannapig_", "ibabyrainbow", "tastelesstv"},
		},
		{
			networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD,
			[]string{"1157956746", "1160400711"},
		},
		// {
		// 	networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
		// 	[]string{},
		// },
	}

	// ts := time.Now().Unix()
	// for i := 0; i < 10; i++ {
	// 	sets[0].ids = append(sets[0].ids, strconv.FormatInt(ts*10+int64(i), 10))
	// }

	eg, ctx := errgroup.WithContext(ctx)

	for _, set := range sets {
		for _, id := range set.ids {
			listing := &networkv1directory.Listing{
				Content: &networkv1directory.Listing_Embed_{
					Embed: &networkv1directory.Listing_Embed{
						Service: set.service,
						Id:      id,
					},
				},
			}

			eg.Go(func() error {
				return directoryClient.Publish(
					ctx,
					&networkv1directory.PublishRequest{Listing: listing},
					&networkv1directory.PublishResponse{},
				)
			})
		}
	}

	if err := eg.Wait(); err != nil {
		return nil, err
	}

	// listing := &networkv1directory.Listing{
	// 	// Content: &networkv1directory.Listing_Media_{
	// 	// 	Media: &networkv1directory.Listing_Media{
	// 	// 		MimeType: rtmpingress.TranscoderMimeType,
	// 	// 		// SwarmUri:  s.swarm.URI().String(),
	// 	// 	},
	// 	// },
	// 	Content: &networkv1directory.Listing_Embed_{
	// 		Embed: &networkv1directory.Listing_Embed{
	// 			Service: networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
	// 			Id:      "psrngafk",
	// 		},
	// 	},
	// }

	// res := &networkv1directory.PublishResponse{}
	// err = directoryClient.Publish(ctx, &networkv1directory.PublishRequest{Listing: listing}, res)
	// if err != nil {
	// 	return nil, err
	// }

	// err = directoryClient.Join(
	// 	ctx,
	// 	&networkv1directory.JoinRequest{Id: res.Id},
	// 	&networkv1directory.JoinResponse{},
	// )
	// if err != nil {
	// 	return nil, err
	// }

	return &networkv1directory.FrontendTestResponse{}, nil
}

func (s *directoryService) ModerateListing(ctx context.Context, r *networkv1directory.FrontendModerateListingRequest) (*networkv1directory.FrontendModerateListingResponse, error) {
	err := s.app.Directory().ModerateListing(ctx, r.Id, r.Moderation, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendModerateListingResponse{}, nil
}

func (s *directoryService) ModerateUser(ctx context.Context, r *networkv1directory.FrontendModerateUserRequest) (*networkv1directory.FrontendModerateUserResponse, error) {
	cert, err := s.app.Network().CA().FindBySubject(ctx, r.NetworkKey, r.Alias)
	if err != nil {
		return nil, err
	}

	err = s.app.Directory().ModerateUser(ctx, cert.Key, r.Moderation, r.NetworkKey)
	if err != nil {
		return nil, err
	}
	return &networkv1directory.FrontendModerateUserResponse{}, nil
}

func (s *directoryService) GetUsers(ctx context.Context, r *networkv1directory.FrontendGetUsersRequest) (*networkv1directory.FrontendGetUsersResponse, error) {
	res := &networkv1directory.FrontendGetUsersResponse{
		Networks: map[uint64]*networkv1directory.Network{},
	}
	users := hashmap.New[[]byte, *networkv1directory.FrontendGetUsersResponse_User](hashmap.NewByteInterface[[]byte]())

	networks, err := dao.Networks.GetAll(s.store)
	if err != nil {
		return nil, err
	}

	for _, n := range networks {
		res.Networks[n.Id] = &networkv1directory.Network{
			Id:   n.Id,
			Name: dao.CertificateRoot(n.Certificate).Subject,
			Key:  dao.NetworkKey(n),
		}

	EachNetworkUser:
		for _, u := range s.app.Directory().GetUsersByNetworkID(n.Id) {
			ru, ok := users.Get(u.PeerKey)
			if !ok {
				ru = &networkv1directory.FrontendGetUsersResponse_User{PeerKey: u.PeerKey}
				users.Set(u.PeerKey, ru)
				res.Users = append(res.Users, ru)
			}

			for _, a := range ru.Aliases {
				if a.Alias == u.Alias {
					a.NetworkIds = append(a.NetworkIds, n.Id)
					continue EachNetworkUser
				}
			}

			ru.Aliases = append(ru.Aliases, &networkv1directory.FrontendGetUsersResponse_Alias{
				Alias:      u.Alias,
				NetworkIds: []uint64{n.Id},
			})
		}
	}

	return res, nil
}

func listingProtoContentType(l *networkv1directory.Listing) networkv1directory.ListingContentType {
	switch l.Content.(type) {
	case *networkv1directory.Listing_Media_:
		return networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_MEDIA
	case *networkv1directory.Listing_Service_:
		return networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_SERVICE
	case *networkv1directory.Listing_Embed_:
		return networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_EMBED
	case *networkv1directory.Listing_Chat_:
		return networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_CHAT
	default:
		return networkv1directory.ListingContentType_LISTING_CONTENT_TYPE_UNDEFINED
	}
}

func (s *directoryService) GetListings(ctx context.Context, r *networkv1directory.FrontendGetListingsRequest) (*networkv1directory.FrontendGetListingsResponse, error) {
	var networks []*networkv1.Network
	var err error
	if len(r.NetworkKeys) == 0 {
		networks, err = dao.Networks.GetAll(s.store)
	} else {
		networks, err = dao.NetworksByKey.GetMany(s.store, r.NetworkKeys...)
	}
	if err != nil {
		return nil, err
	}

	res := &networkv1directory.FrontendGetListingsResponse{}

	for _, n := range networks {
		var nls []*networkv1directory.NetworkListingsItem
		for _, l := range s.app.Directory().GetListingsByNetworkID(n.Id) {
			if len(r.ContentTypes) == 0 || slices.Contains(r.ContentTypes, listingProtoContentType(l.Listing)) {
				nls = append(nls, &networkv1directory.NetworkListingsItem{
					Id:         l.ID,
					Listing:    l.Listing,
					Snippet:    l.Snippet,
					Moderation: l.Moderation,
					UserCount:  l.UserCount,
				})
			}
		}

		res.Listings = append(res.Listings, &networkv1directory.NetworkListings{
			Network: &networkv1directory.Network{
				Id:   n.Id,
				Name: dao.CertificateRoot(n.Certificate).Subject,
				Key:  dao.NetworkKey(n),
			},
			Listings: nls,
		})
	}

	return res, nil
}

func (s *directoryService) WatchListings(ctx context.Context, r *networkv1directory.FrontendWatchListingsRequest) (<-chan *networkv1directory.FrontendWatchListingsResponse, error) {
	var networks []*networkv1.Network
	var err error
	if len(r.NetworkKeys) == 0 {
		networks, err = dao.Networks.GetAll(s.store)
	} else {
		networks, err = dao.NetworksByKey.GetMany(s.store, r.NetworkKeys...)
	}
	if err != nil {
		return nil, err
	}

	events := make(chan *networkv1directory.FrontendWatchListingsResponse_Event, 128)

	var errs []error
	for _, n := range networks {
		network := &networkv1directory.Network{
			Id:   n.Id,
			Name: dao.CertificateRoot(n.Certificate).Subject,
			Key:  dao.NetworkKey(n),
		}

		src := make(chan directory.ListingEvent, 128)
		listings, err := s.app.Directory().NotifyListingEvent(network.Id, src)
		if err != nil {
			errs = append(errs, err)
		}

		go func() {
			defer s.app.Directory().StopNotifyingListingEvent(network.Id, src)

			var nls []*networkv1directory.NetworkListingsItem
			for _, l := range listings {
				if len(r.ContentTypes) == 0 || slices.Contains(r.ContentTypes, listingProtoContentType(l.Listing)) {
					nls = append(nls, &networkv1directory.NetworkListingsItem{
						Id:         l.ID,
						Listing:    l.Listing,
						Snippet:    l.Snippet,
						Moderation: l.Moderation,
						UserCount:  l.UserCount,
					})
				}
			}
			res := &networkv1directory.FrontendWatchListingsResponse_Event{
				Event: &networkv1directory.FrontendWatchListingsResponse_Event_Change{
					Change: &networkv1directory.FrontendWatchListingsResponse_Change{
						Listings: &networkv1directory.NetworkListings{
							Network:  network,
							Listings: nls,
						},
					},
				},
			}
			select {
			case <-ctx.Done():
				return
			case events <- res:
			}

			for {
				select {
				case <-ctx.Done():
					return
				case e, ok := <-src:
					if !ok {
						return
					}

					l := e.Listing
					if len(r.ContentTypes) != 0 && !slices.Contains(r.ContentTypes, listingProtoContentType(l.Listing)) {
						continue
					}

					res := &networkv1directory.FrontendWatchListingsResponse_Event{}
					switch e.Type {
					case directory.ChangeListingEventType:
						res.Event = &networkv1directory.FrontendWatchListingsResponse_Event_Change{
							Change: &networkv1directory.FrontendWatchListingsResponse_Change{
								Listings: &networkv1directory.NetworkListings{
									Network: network,
									Listings: []*networkv1directory.NetworkListingsItem{{
										Id:         l.ID,
										Listing:    l.Listing,
										Snippet:    l.Snippet,
										Moderation: l.Moderation,
										UserCount:  l.UserCount,
									}},
								},
							},
						}
					case directory.UnpublishListingEventType:
						res.Event = &networkv1directory.FrontendWatchListingsResponse_Event_Unpublish{
							Unpublish: &networkv1directory.FrontendWatchListingsResponse_Unpublish{
								NetworkId: network.Id,
								ListingId: l.ID,
							},
						}
					case directory.UserCountChangeListingEventType:
						res.Event = &networkv1directory.FrontendWatchListingsResponse_Event_UserCountChange{
							UserCountChange: &networkv1directory.FrontendWatchListingsResponse_UserCountChange{
								NetworkId: network.Id,
								ListingId: l.ID,
								UserCount: l.UserCount,
							},
						}
					}

					select {
					case <-ctx.Done():
						return
					case events <- res:
					}
				}
			}
		}()
	}
	if errs != nil {
		return nil, multierr.Combine(errs...)
	}

	ch := make(chan *networkv1directory.FrontendWatchListingsResponse)

	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			case e := <-events:
				res := &networkv1directory.FrontendWatchListingsResponse{
					Events: chanutil.AppendAll([]*networkv1directory.FrontendWatchListingsResponse_Event{e}, events),
				}
				select {
				case <-ctx.Done():
					return
				case ch <- res:
				}
			}
		}
	}()

	return ch, nil
}

func (s *directoryService) WatchListingUsers(ctx context.Context, r *networkv1directory.FrontendWatchListingUsersRequest) (<-chan *networkv1directory.FrontendWatchListingUsersResponse, error) {
	networkID, err := dao.NetworksByKey.GetID(s.store, r.NetworkKey)
	if err != nil {
		return nil, err
	}

	listing, ok := s.app.Directory().GetListingByQuery(networkID, r.Query)
	if !ok {
		return nil, errors.New("listing not found")
	}

	events := make(chan directory.UserEvent, 1)
	users, err := s.app.Directory().NotifyListingUserEvent(networkID, listing.ID, events)
	if err != nil {
		return nil, err
	}

	ch := make(chan *networkv1directory.FrontendWatchListingUsersResponse, 1)
	go func() {
		defer s.app.Directory().StopNotifyingListingUserEvent(networkID, listing.ID, events)

		res := &networkv1directory.FrontendWatchListingUsersResponse{
			Type: networkv1directory.FrontendWatchListingUsersResponse_USER_EVENT_TYPE_JOIN,
		}
		for _, u := range users {
			res.Users = append(res.Users, &networkv1directory.FrontendWatchListingUsersResponse_User{
				Id:      u.ID,
				Alias:   u.Alias,
				PeerKey: u.PeerKey,
			})
		}
		ch <- res

		for {
			select {
			case <-ctx.Done():
				return
			case e, ok := <-events:
				if !ok {
					return
				}

				res := &networkv1directory.FrontendWatchListingUsersResponse{
					Users: []*networkv1directory.FrontendWatchListingUsersResponse_User{{
						Id:      e.User.ID,
						Alias:   e.User.Alias,
						PeerKey: e.User.PeerKey,
					}},
				}
				switch e.Type {
				case directory.JoinUserEventType:
					res.Type = networkv1directory.FrontendWatchListingUsersResponse_USER_EVENT_TYPE_JOIN
				case directory.PartUserEventType:
					res.Type = networkv1directory.FrontendWatchListingUsersResponse_USER_EVENT_TYPE_PART
				case directory.RenameUserEventType:
					res.Type = networkv1directory.FrontendWatchListingUsersResponse_USER_EVENT_TYPE_RENAME
				}

				select {
				case <-ctx.Done():
					return
				case ch <- res:
				}
			}
		}
	}()

	return ch, nil
}
