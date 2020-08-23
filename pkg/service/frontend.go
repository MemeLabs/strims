package service

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"runtime"
	"runtime/pprof"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const metadataTable = "default"

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
	ErrAuthenticationRequired  = errors.New("method requires authentication")
	ErrAlreadyJoinedNetwork    = errors.New("user already has a membership for that network")
)

// Options ...
type Options struct {
	Store      kv.BlobStore
	Logger     *zap.Logger
	VPNOptions []vpn.HostOption
}

// New ...
func New(options Options) (*Frontend, error) {
	metadata, err := dao.NewMetadataStore(options.Store)
	if err != nil {
		return nil, err
	}

	return &Frontend{
		logger:     options.Logger,
		store:      options.Store,
		metadata:   metadata,
		vpnOptions: options.VPNOptions,
	}, nil
}

// Frontend ...
type Frontend struct {
	logger     *zap.Logger
	store      kv.BlobStore
	metadata   *dao.MetadataStore
	vpnOptions []vpn.HostOption
}

// CreateProfile ...
func (s *Frontend) CreateProfile(ctx context.Context, r *pb.CreateProfileRequest) (*pb.CreateProfileResponse, error) {
	profile, store, err := dao.CreateProfile(s.metadata, r.Name, r.Password)
	if err != nil {
		return nil, err
	}

	session := rpc.ContextSession(ctx)
	session.Init(profile, store)

	return &pb.CreateProfileResponse{
		SessionId: session.ID(),
		Profile:   profile,
	}, nil
}

// DeleteProfile ...
func (s *Frontend) DeleteProfile(ctx context.Context, r *pb.DeleteProfileRequest) (*pb.DeleteProfileResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	if err := dao.DeleteProfile(s.metadata, session.Profile()); err != nil {
		return nil, err
	}

	if err := session.ProfileStore().Delete(); err != nil {
		return nil, err
	}

	session.Reset()

	return &pb.DeleteProfileResponse{}, nil
}

// LoadProfile ...
func (s *Frontend) LoadProfile(ctx context.Context, r *pb.LoadProfileRequest) (*pb.LoadProfileResponse, error) {
	profile, store, err := dao.LoadProfile(s.metadata, r.Id, r.Password)
	if err != nil {
		return nil, err
	}

	session := rpc.ContextSession(ctx)
	session.Init(profile, store)

	return &pb.LoadProfileResponse{
		SessionId: session.ID(),
		Profile:   profile,
	}, nil
}

// LoadSession ...
func (s *Frontend) LoadSession(ctx context.Context, r *pb.LoadSessionRequest) (*pb.LoadSessionResponse, error) {
	id, storageKey, err := rpc.UnmarshalSessionID(r.SessionId)
	if err != nil {
		return nil, err
	}

	profile, store, err := dao.LoadProfileFromSession(s.metadata, id, storageKey)
	if err != nil {
		return nil, err
	}

	session := rpc.ContextSession(ctx)
	session.Init(profile, store)

	return &pb.LoadSessionResponse{
		SessionId: session.ID(),
		Profile:   profile,
	}, nil
}

// GetProfile ...
func (s *Frontend) GetProfile(ctx context.Context, r *pb.GetProfileRequest) (*pb.GetProfileResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	profile, err := dao.GetProfile(session.ProfileStore())
	if err != nil {
		return nil, err
	}

	return &pb.GetProfileResponse{Profile: profile}, nil
}

// GetProfiles ...
func (s *Frontend) GetProfiles(ctx context.Context, r *pb.GetProfilesRequest) (*pb.GetProfilesResponse, error) {
	profiles, err := dao.GetProfileSummaries(s.metadata)
	if err != nil {
		return nil, err
	}

	return &pb.GetProfilesResponse{Profiles: profiles}, nil
}

// CreateNetwork ...
func (s *Frontend) CreateNetwork(ctx context.Context, r *pb.CreateNetworkRequest) (*pb.CreateNetworkResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	network, err := dao.NewNetwork(r.Name)
	if err != nil {
		return nil, err
	}

	csr, err := dao.NewCertificateRequest(session.Profile().Key, pb.KeyUsage_KEY_USAGE_PEER)
	if err != nil {
		return nil, err
	}
	membership, err := dao.NewNetworkMembershipFromNetwork(network, csr)
	if err != nil {
		return nil, err
	}

	err = session.ProfileStore().Update(func(tx kv.RWTx) error {
		if err := dao.InsertNetwork(tx, network); err != nil {
			return err
		}

		return dao.InsertNetworkMembership(tx, membership)
	})
	if err != nil {
		return nil, err
	}

	err = s.startNetwork(ctx, membership, network)
	if err != nil {
		return nil, err
	}

	return &pb.CreateNetworkResponse{Network: network}, nil
}

// DeleteNetwork ...
func (s *Frontend) DeleteNetwork(ctx context.Context, r *pb.DeleteNetworkRequest) (*pb.DeleteNetworkResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	err := session.ProfileStore().Update(func(tx kv.RWTx) error {
		membership, err := dao.GetNetworkMembershipForNetwork(tx, r.Id)
		if err != nil && err != kv.ErrRecordNotFound {
			return err
		}

		if membership != nil {
			if err := dao.DeleteNetworkMembership(tx, membership.Id); err != nil {
				return err
			}
		}

		if err := dao.DeleteNetwork(tx, r.Id); err != nil {
			if err == kv.ErrRecordNotFound {
				return fmt.Errorf("could not delete network: %w", err)
			}
			return err
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	return &pb.DeleteNetworkResponse{}, nil
}

// GetNetwork ...
func (s *Frontend) GetNetwork(ctx context.Context, r *pb.GetNetworkRequest) (*pb.GetNetworkResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	network, err := dao.GetNetwork(session.ProfileStore(), r.Id)
	if err != nil {
		return nil, err
	}
	return &pb.GetNetworkResponse{Network: network}, nil
}

// GetNetworks ...
func (s *Frontend) GetNetworks(ctx context.Context, r *pb.GetNetworksRequest) (*pb.GetNetworksResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	networks, err := dao.GetNetworks(session.ProfileStore())
	if err != nil {
		return nil, err
	}
	return &pb.GetNetworksResponse{Networks: networks}, nil
}

// GetNetworkMemberships ...
func (s *Frontend) GetNetworkMemberships(ctx context.Context, r *pb.GetNetworkMembershipsRequest) (*pb.GetNetworkMembershipsResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	memberships, err := dao.GetNetworkMemberships(session.ProfileStore())
	if err != nil {
		return nil, err
	}
	return &pb.GetNetworkMembershipsResponse{NetworkMemberships: memberships}, nil
}

// DeleteNetworkMembership ...
func (s *Frontend) DeleteNetworkMembership(ctx context.Context, r *pb.DeleteNetworkMembershipRequest) (*pb.DeleteNetworkMembershipResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	membership, err := dao.GetNetworkMembership(session.ProfileStore(), r.Id)
	if err != nil {
		if err == kv.ErrRecordNotFound {
			return nil, fmt.Errorf("could not delete network membership: %w", err)
		}
		return nil, err
	}
	controller, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	if err := controller.StopNetwork(membership.Certificate); err != nil {
		return nil, err
	}

	if err := dao.DeleteNetworkMembership(session.ProfileStore(), r.Id); err != nil {
		return nil, err
	}
	return &pb.DeleteNetworkMembershipResponse{}, nil
}

// CreateBootstrapClient ...
func (s *Frontend) CreateBootstrapClient(ctx context.Context, r *pb.CreateBootstrapClientRequest) (*pb.CreateBootstrapClientResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	var client *pb.BootstrapClient
	var err error
	switch v := r.GetClientOptions().(type) {
	case *pb.CreateBootstrapClientRequest_WebsocketOptions:
		client, err = dao.NewWebSocketBootstrapClient(v.WebsocketOptions.Url, v.WebsocketOptions.InsecureSkipVerifyTls)
	}
	if err != nil {
		return nil, err
	}

	if err := dao.InsertBootstrapClient(session.ProfileStore(), client); err != nil {
		return nil, err
	}

	return &pb.CreateBootstrapClientResponse{BootstrapClient: client}, nil
}

// UpdateBootstrapClient ...
func (s *Frontend) UpdateBootstrapClient(ctx context.Context, r *pb.UpdateBootstrapClientRequest) (*pb.UpdateBootstrapClientResponse, error) {

	return &pb.UpdateBootstrapClientResponse{BootstrapClient: nil}, nil
}

// DeleteBootstrapClient ...
func (s *Frontend) DeleteBootstrapClient(ctx context.Context, r *pb.DeleteBootstrapClientRequest) (*pb.DeleteBootstrapClientResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	if err := dao.DeleteBootstrapClient(session.ProfileStore(), r.Id); err != nil {
		return nil, err
	}

	return &pb.DeleteBootstrapClientResponse{}, nil
}

// GetBootstrapClient ...
func (s *Frontend) GetBootstrapClient(ctx context.Context, r *pb.GetBootstrapClientRequest) (*pb.GetBootstrapClientResponse, error) {
	return &pb.GetBootstrapClientResponse{BootstrapClient: nil}, nil
}

// GetBootstrapClients ...
func (s *Frontend) GetBootstrapClients(ctx context.Context, r *pb.GetBootstrapClientsRequest) (*pb.GetBootstrapClientsResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	clients, err := dao.GetBootstrapClients(session.ProfileStore())
	if err != nil {
		return nil, err
	}

	return &pb.GetBootstrapClientsResponse{BootstrapClients: clients}, nil
}

// GetBootstrapPeers ...
func (s *Frontend) GetBootstrapPeers(ctx context.Context, r *pb.GetBootstrapPeersRequest) (*pb.GetBootstrapPeersResponse, error) {
	svc, err := s.getBootstrapService(ctx)
	if err != nil {
		return nil, err
	}

	peers := []*pb.BootstrapPeer{}
	for _, id := range svc.GetPeerHostIDs() {
		peers = append(peers, &pb.BootstrapPeer{
			HostId: id.Bytes(nil),
			Label:  hex.EncodeToString(id.Bytes(nil)),
		})
	}

	return &pb.GetBootstrapPeersResponse{Peers: peers}, nil
}

// PublishNetworkToBootstrapPeer ...
func (s *Frontend) PublishNetworkToBootstrapPeer(ctx context.Context, r *pb.PublishNetworkToBootstrapPeerRequest) (*pb.PublishNetworkToBootstrapPeerResponse, error) {
	svc, err := s.getBootstrapService(ctx)
	if err != nil {
		return nil, err
	}

	id, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return nil, err
	}

	if err := svc.PublishNetwork(id, r.Network); err != nil {
		return nil, err
	}

	return &pb.PublishNetworkToBootstrapPeerResponse{}, nil
}

// CreateChatServer ...
func (s *Frontend) CreateChatServer(ctx context.Context, r *pb.CreateChatServerRequest) (*pb.CreateChatServerResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	server, err := dao.NewChatServer(r.NetworkKey, r.ChatRoom)
	if err != nil {
		return nil, err
	}

	if err := dao.InsertChatServer(session.ProfileStore(), server); err != nil {
		return nil, err
	}

	return &pb.CreateChatServerResponse{ChatServer: server}, nil
}

// UpdateChatServer ...
func (s *Frontend) UpdateChatServer(ctx context.Context, r *pb.UpdateChatServerRequest) (*pb.UpdateChatServerResponse, error) {

	return &pb.UpdateChatServerResponse{ChatServer: nil}, nil
}

// DeleteChatServer ...
func (s *Frontend) DeleteChatServer(ctx context.Context, r *pb.DeleteChatServerRequest) (*pb.DeleteChatServerResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	if err := dao.DeleteChatServer(session.ProfileStore(), r.Id); err != nil {
		return nil, err
	}

	return &pb.DeleteChatServerResponse{}, nil
}

// GetChatServer ...
func (s *Frontend) GetChatServer(ctx context.Context, r *pb.GetChatServerRequest) (*pb.GetChatServerResponse, error) {
	return &pb.GetChatServerResponse{ChatServer: nil}, nil
}

// GetChatServers ...
func (s *Frontend) GetChatServers(ctx context.Context, r *pb.GetChatServersRequest) (*pb.GetChatServersResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	servers, err := dao.GetChatServers(session.ProfileStore())
	if err != nil {
		return nil, err
	}

	return &pb.GetChatServersResponse{ChatServers: servers}, nil
}

// JoinSwarm ...
func (s *Frontend) JoinSwarm(ctx context.Context, r *pb.JoinSwarmRequest) (*pb.JoinSwarmResponse, error) {
	return &pb.JoinSwarmResponse{}, nil
}

// LeaveSwarm ...
func (s *Frontend) LeaveSwarm(ctx context.Context, r *pb.LeaveSwarmRequest) (*pb.LeaveSwarmResponse, error) {
	return &pb.LeaveSwarmResponse{}, nil
}

// BootstrapDHT ...
func (s *Frontend) BootstrapDHT(ctx context.Context, r *pb.BootstrapDHTRequest) (*pb.BootstrapDHTResponse, error) {
	return &pb.BootstrapDHTResponse{}, nil
}

type vpnKeyType struct{}

var vpnKey = vpnKeyType{}

type vpnData struct {
	host             *vpn.Host
	controller       *NetworkController
	bootstrapService *BootstrapService
}

// StartVPN ...
func (s *Frontend) StartVPN(ctx context.Context, r *pb.StartVPNRequest) (<-chan *pb.NetworkEvent, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	// TODO: locking...
	if _, ok := session.Values.Load(vpnKey); ok {
		return nil, errors.New("vpn already running")
	}

	profile, err := dao.GetProfile(session.ProfileStore())
	if err != nil {
		return nil, err
	}

	host, err := vpn.NewHost(s.logger, profile.Key, s.vpnOptions...)
	if err != nil {
		return nil, err
	}

	if err := StartBootstrapClients(s.logger, host, session.ProfileStore()); err != nil {
		return nil, err
	}

	networkController, err := NewNetworkController(s.logger, host, session.ProfileStore())
	if err != nil {
		return nil, err
	}

	bootstrapService := NewBootstrapService(
		s.logger,
		host,
		session.ProfileStore(),
		networkController,
		BootstrapServiceOptions{
			EnablePublishing: r.EnableBootstrapPublishing,
		},
	)

	session.Values.Store(vpnKey, vpnData{host, networkController, bootstrapService})

	return networkController.Events(), nil
}

// StopVPN ...
func (s *Frontend) StopVPN(ctx context.Context, r *pb.StopVPNRequest) (*pb.StopVPNResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	if vi, ok := session.Values.Load(vpnKey); ok {
		v := vi.(vpnData)
		v.host.Close()
		session.Values.Delete(vpnKey)
	}

	return &pb.StopVPNResponse{}, nil
}

// OpenVideoClient ...
func (s *Frontend) OpenVideoClient(ctx context.Context, r *pb.VideoClientOpenRequest) (<-chan *pb.VideoClientEvent, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	s.logger.Debug("start swarm...")

	v, err := NewVideoClient()
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
func (s *Frontend) OpenVideoServer(ctx context.Context, r *pb.VideoServerOpenRequest) (*pb.VideoServerOpenResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	s.logger.Debug("start swarm...")

	v, err := NewVideoServer(s.logger)
	if err != nil {
		return nil, err
	}

	id := session.Store(v)

	return &pb.VideoServerOpenResponse{Id: id}, nil
}

// WriteToVideoServer ...
func (s *Frontend) WriteToVideoServer(ctx context.Context, r *pb.VideoServerWriteRequest) (*pb.VideoServerWriteResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
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

	return &pb.VideoServerWriteResponse{}, nil
}

// PublishSwarm ...
func (s *Frontend) PublishSwarm(ctx context.Context, r *pb.PublishSwarmRequest) (*pb.PublishSwarmResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
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

// StopSwarm ...
// func (s *Frontend) StopSwarm(ctx context.Context, r *pb.StopSwarmRequest) (*pb.StopSwarmResponse, error) {
// 	w, ok := s.swarmWriters[r.Id]
// 	if !ok {
// 		return nil, errors.New("swarm id not found")
// 	}
// 	delete(s.swarmWriters, r.Id)

// 	if err := w.Flush(); err != nil {
// 		return nil, err
// 	}
// 	if err := w.Close(); err != nil {
// 		return nil, err
// 	}

// 	return &pb.StopSwarmResponse{}, nil
// }

// PProf ...
func (s *Frontend) PProf(ctx context.Context, r *pb.PProfRequest) (*pb.PProfResponse, error) {
	p := pprof.Lookup(r.Name)
	if p == nil {
		return nil, errors.New("unknown profile")
	}
	if r.Name == "heap" && r.Gc {
		runtime.GC()
	}

	b := &bytes.Buffer{}

	var debug int
	if r.Debug {
		debug = 1
	}
	if err := p.WriteTo(b, debug); err != nil {
		return nil, err
	}

	return &pb.PProfResponse{Name: r.Name, Data: b.Bytes()}, nil
}

// OpenChatServer ...
func (s *Frontend) OpenChatServer(ctx context.Context, r *pb.OpenChatServerRequest) (<-chan *pb.ChatServerEvent, error) {
	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	ch := make(chan *pb.ChatServerEvent, 1)

	// TODO: this should return an ErrNetworkNotFound...
	svc, ok := ctl.NetworkServices(r.Server.NetworkKey)
	if !ok {
		return nil, errors.New("unknown network")
	}

	session := rpc.ContextSession(ctx)

	server, err := NewChatServer(s.logger, svc, r.Server.Key)
	if err != nil {
		return nil, err
	}

	id := session.Store(server)
	ch <- &pb.ChatServerEvent{
		Body: &pb.ChatServerEvent_Open_{
			Open: &pb.ChatServerEvent_Open{
				ServerId: id,
			},
		},
	}

	go func() {
		for e := range server.Events() {
			ch <- e
		}
		ch <- &pb.ChatServerEvent{
			Body: &pb.ChatServerEvent_Close_{
				Close: &pb.ChatServerEvent_Close{},
			},
		}

		session.Delete(id)
		close(ch)
	}()

	return ch, nil
}

// CallChatServer ...
func (s *Frontend) CallChatServer(ctx context.Context, r *pb.CallChatServerRequest) error {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return ErrAuthenticationRequired
	}

	serverIf, _ := session.Load(r.ServerId)
	server, ok := serverIf.(*ChatServer)
	if !ok {
		return errors.New("server id does not exist")
	}

	switch r.Body.(type) {
	case *pb.CallChatServerRequest_Close_:
		server.Close()
	}

	return nil
}

// OpenChatClient ...
func (s *Frontend) OpenChatClient(ctx context.Context, r *pb.OpenChatClientRequest) (<-chan *pb.ChatClientEvent, error) {
	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	ch := make(chan *pb.ChatClientEvent, 1)

	// TODO: this should return an ErrNetworkNotFound...
	svc, ok := ctl.NetworkServices(r.NetworkKey)
	if !ok {
		return nil, errors.New("unknown network")
	}

	session := rpc.ContextSession(ctx)

	client, err := NewChatClient(s.logger, svc, r.ServerKey)
	if err != nil {
		return nil, err
	}

	id := session.Store(client)
	ch <- &pb.ChatClientEvent{
		Body: &pb.ChatClientEvent_Open_{
			Open: &pb.ChatClientEvent_Open{
				ClientId: id,
			},
		},
	}

	go func() {
		for e := range client.Events() {
			ch <- e
		}
		ch <- &pb.ChatClientEvent{
			Body: &pb.ChatClientEvent_Close_{
				Close: &pb.ChatClientEvent_Close{},
			},
		}

		session.Delete(id)
		close(ch)
	}()

	return ch, nil
}

// CallChatClient ...
func (s *Frontend) CallChatClient(ctx context.Context, r *pb.CallChatClientRequest) error {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return ErrAuthenticationRequired
	}

	clientIf, _ := session.Load(r.ClientId)
	client, ok := clientIf.(*ChatClient)
	if !ok {
		return errors.New("client id does not exist")
	}

	switch b := r.Body.(type) {
	case *pb.CallChatClientRequest_Message_:
		if err := client.Send(&pb.ChatClientEvent_Message{
			Body: b.Message.Body,
		}); err != nil {
			return err
		}
	case *pb.CallChatClientRequest_Close_:
		client.Close()
	}

	return nil
}

// CreateNetworkInvitation ...
func (s *Frontend) CreateNetworkInvitation(ctx context.Context, r *pb.CreateNetworkInvitationRequest) (*pb.CreateNetworkInvitationResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	key, err := dao.GenerateKey()
	if err != nil {
		return nil, err
	}

	validDuration := time.Hour * 24 * 7 // seven days
	hostKey := r.SigningKey
	signingCert := r.SigningCert

	inviteCSR := &pb.CertificateRequest{
		Key:      key.Public,
		KeyType:  pb.KeyType_KEY_TYPE_ED25519,
		KeyUsage: uint32(pb.KeyUsage_KEY_USAGE_BOOTSTRAP | pb.KeyUsage_KEY_USAGE_SIGN),
	}
	inviteCert, err := dao.SignCertificateRequest(inviteCSR, validDuration, hostKey)
	if err != nil {
		return nil, err
	}
	inviteCert.ParentOneof = &pb.Certificate_Parent{Parent: signingCert}

	b, err := proto.Marshal(&pb.InvitationV0{
		Key:         key,
		Certificate: inviteCert,
		NetworkName: r.NetworkName,
	})
	if err != nil {
		return nil, err
	}

	b, err = proto.Marshal(&pb.Invitation{
		Version: 0,
		Data:    b,
	})
	if err != nil {
		return nil, err
	}

	b64 := base64.StdEncoding.WithPadding(base64.NoPadding).EncodeToString(b)

	return &pb.CreateNetworkInvitationResponse{
		InvitationB64:   b64,
		InvitationBytes: b,
	}, nil
}

// CreateNetworkMembershipFromInvitation ...
func (s *Frontend) CreateNetworkMembershipFromInvitation(ctx context.Context, r *pb.CreateNetworkMembershipFromInvitationRequest) (*pb.CreateNetworkMembershipFromInvitationResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	var invBytes []byte

	switch x := r.Invitation.(type) {
	case *pb.CreateNetworkMembershipFromInvitationRequest_InvitationB64:
		var err error
		invBytes, err = base64.StdEncoding.WithPadding(base64.NoPadding).DecodeString(r.GetInvitationB64())
		if err != nil {
			return nil, err
		}
	case *pb.CreateNetworkMembershipFromInvitationRequest_InvitationBytes:
		invBytes = r.GetInvitationBytes()
	case nil:
		return nil, errors.New("Invitation has no content")
	default:
		return nil, fmt.Errorf("Invitation has unexpected type %T", x)
	}

	var wrapper pb.Invitation
	err := proto.Unmarshal(invBytes, &wrapper)
	if err != nil {
		return nil, err
	}

	var invitation pb.InvitationV0
	err = proto.Unmarshal(wrapper.Data, &invitation)
	if err != nil {
		return nil, err
	}

	err = dao.VerifyCertificate(invitation.Certificate)
	if err != nil {
		return nil, err
	}

	inviteCSR := &pb.CertificateRequest{
		Key:      session.Profile().Key.Public,
		KeyType:  pb.KeyType_KEY_TYPE_ED25519,
		KeyUsage: uint32(pb.KeyUsage_KEY_USAGE_PEER),
	}

	membership, err := dao.NewNetworkMembershipFromInvite(&invitation, inviteCSR)
	if err != nil {
		return nil, err
	}

	err = s.saveNetworkMembership(ctx, membership)
	if err != nil {
		return nil, err
	}

	err = s.startNetwork(ctx, membership, nil)
	if err != nil {
		return nil, err
	}

	return &pb.CreateNetworkMembershipFromInvitationResponse{
		Membership: membership,
	}, nil
}

// saveNetworkMembership saves a network membership.
// it returns an error if the user is already has a valid membership for that network
func (s *Frontend) saveNetworkMembership(ctx context.Context, membership *pb.NetworkMembership) error {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return ErrAuthenticationRequired
	}

	old, err := dao.GetNetworkMembershipByNetworkKey(session.ProfileStore(), dao.GetRootCert(membership.Certificate).Key)
	if err != nil && err != kv.ErrRecordNotFound {
		return err
	}
	if old != nil {
		if !dao.CertIsExpired(old.Certificate) {
			return ErrAlreadyJoinedNetwork
		}

		if err := dao.DeleteNetworkMembership(session.ProfileStore(), old.Id); err != nil {
			return fmt.Errorf("could not delete old membership: %w", err)
		}
	}

	return dao.InsertNetworkMembership(session.ProfileStore(), membership)
}

func (s *Frontend) startNetwork(ctx context.Context, membership *pb.NetworkMembership, network *pb.Network) error {
	controller, err := s.getNetworkController(ctx)
	if err != nil {
		return err
	}

	// TODO: restart/update network with new cert?
	if network == nil {
		_, err = controller.StartNetwork(membership.Certificate, WithMemberServices(s.logger))
	} else {
		store := rpc.ContextSession(ctx).ProfileStore()
		_, err = controller.StartNetwork(membership.Certificate, WithOwnerServices(s.logger, store, network))
	}
	return err
}

// loads the NetworkController fron the session store
// TODO: move to (s *Session) getNetworkController ?
func (s *Frontend) getNetworkController(ctx context.Context) (*NetworkController, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	d, ok := session.Values.Load(vpnKey)
	if !ok {
		return nil, errors.New("could not get vpn data")
	}

	data, ok := d.(vpnData)
	if !ok {
		return nil, errors.New("vpn data has unexpected type")
	}

	return data.controller, nil
}

func (s *Frontend) getBootstrapService(ctx context.Context) (*BootstrapService, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	d, ok := session.Values.Load(vpnKey)
	if !ok {
		return nil, errors.New("could not get vpn data")
	}

	data, ok := d.(vpnData)
	if !ok {
		return nil, errors.New("vpn data has unexpected type")
	}

	return data.bootstrapService, nil
}
