package service

import (
	"bytes"
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"runtime"
	"runtime/pprof"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
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
	Store      dao.Store
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
	store      dao.Store
	metadata   *dao.MetadataStore
	vpnOptions []vpn.HostOption
}

// CreateProfile ...
func (s *Frontend) CreateProfile(ctx context.Context, r *pb.CreateProfileRequest) (*pb.CreateProfileResponse, error) {
	profile, store, err := s.metadata.CreateProfile(r.Name, r.Password)
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

	if err := s.metadata.DeleteProfile(session.Profile()); err != nil {
		return nil, err
	}

	if err := session.Store().Delete(); err != nil {
		return nil, err
	}

	session.Reset()

	return &pb.DeleteProfileResponse{}, nil
}

// LoadProfile ...
func (s *Frontend) LoadProfile(ctx context.Context, r *pb.LoadProfileRequest) (*pb.LoadProfileResponse, error) {
	profile, store, err := s.metadata.LoadProfile(r.Id, r.Password)
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

	profile, store, err := s.metadata.LoadSession(id, storageKey)
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

	profile, err := session.Store().GetProfile()
	if err != nil {
		return nil, err
	}

	return &pb.GetProfileResponse{Profile: profile}, nil
}

// GetProfiles ...
func (s *Frontend) GetProfiles(ctx context.Context, r *pb.GetProfilesRequest) (*pb.GetProfilesResponse, error) {
	profiles, err := s.metadata.GetProfiles()
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
	if err := session.Store().InsertNetwork(network); err != nil {
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
	if err := session.Store().InsertNetworkMembership(membership); err != nil {
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

	if err := session.Store().DeleteNetwork(r.Id); err != nil {
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

	network, err := session.Store().GetNetwork(r.Id)
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

	networks, err := session.Store().GetNetworks()
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

	memberships, err := session.Store().GetNetworkMemberships()
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

	if err := session.Store().DeleteNetworkMembership(r.Id); err != nil {
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
		client, err = dao.NewWebSocketBootstrapClient(v.WebsocketOptions.Url)
	}
	if err != nil {
		return nil, err
	}

	if err := session.Store().InsertBootstrapClient(client); err != nil {
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

	if err := session.Store().DeleteBootstrapClient(r.Id); err != nil {
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

	clients, err := session.Store().GetBootstrapClients()
	if err != nil {
		return nil, err
	}

	return &pb.GetBootstrapClientsResponse{BootstrapClients: clients}, nil
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

	if err := session.Store().InsertChatServer(server); err != nil {
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

	if err := session.Store().DeleteChatServer(r.Id); err != nil {
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

	servers, err := session.Store().GetChatServers()
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
	host       *vpn.Host
	controller *NetworksController
}

// StartVPN ...
func (s *Frontend) StartVPN(ctx context.Context, r *pb.StartVPNRequest) (*pb.StartVPNResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	// TODO: locking...
	if _, ok := session.Values.Load(vpnKey); !ok {
		hostOptions := append([]vpn.HostOption{}, s.vpnOptions...)

		profile, err := session.Store().GetProfile()
		if err != nil {
			return nil, err
		}

		controller := NewNetworksController(s.logger, session.Store())
		hostOptions = append(hostOptions, WithNetworkController(controller))

		clients, err := session.Store().GetBootstrapClients()
		if err != nil {
			return nil, err
		}
		hostOptions = append(hostOptions, vpn.WithBootstrapClients(clients))

		host, err := vpn.NewHost(s.logger, profile.Key, hostOptions...)
		if err != nil {
			return nil, err
		}

		session.Values.Store(vpnKey, vpnData{host, controller})
	}

	return &pb.StartVPNResponse{}, nil
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

	v := NewVideoThing()

	ch := make(chan *pb.VideoClientEvent, 1)
	go v.RunClient(ch)

	id := atomic.AddUint32(&idThing, 1)
	session.Values.Store(id, v)

	ch <- &pb.VideoClientEvent{
		Body: &pb.VideoClientEvent_Open_{
			Open: &pb.VideoClientEvent_Open{
				Id: id,
			},
		},
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

	v := NewVideoThing()
	v.RunServer()

	id := atomic.AddUint32(&idThing, 1)
	session.Values.Store(id, v)

	return &pb.VideoServerOpenResponse{Id: id}, nil
}

// WriteToVideoServer ...
func (s *Frontend) WriteToVideoServer(ctx context.Context, r *pb.VideoServerWriteRequest) (*pb.VideoServerWriteResponse, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	tif, _ := session.Values.Load(r.Id)
	t, ok := tif.(*VideoThing)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	if err := t.Write(r.Data); err != nil {
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

	tif, _ := session.Values.Load(r.Id)
	t, ok := tif.(*VideoThing)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	vpnDataIf, ok := session.Values.Load(vpnKey)
	if !ok {
		return nil, errors.New("vpnData does not exist")
	}

	if err := t.PublishSwarm(vpnDataIf.(vpnData).controller.ServiceOptions(1)); err != nil {
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
	p.WriteTo(b, debug)

	return &pb.PProfResponse{Name: r.Name, Data: b.Bytes()}, nil
}

// OpenChatClient ...
func (s *Frontend) OpenChatClient(ctx context.Context, r *pb.ChatClientOpenRequest) (<-chan *pb.ChatClientEvent, error) {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return nil, ErrAuthenticationRequired
	}

	ch := make(chan *pb.ChatClientEvent, 1)

	vpnDataIf, ok := session.Values.Load(vpnKey)
	if !ok {
		return nil, errors.New("vpnData does not exist")
	}

	c := NewChatThing(ch, vpnDataIf.(vpnData))

	id := atomic.AddUint32(&idThing, 1)
	session.Values.Store(id, c)

	ch <- &pb.ChatClientEvent{
		Body: &pb.ChatClientEvent_Open_{
			Open: &pb.ChatClientEvent_Open{
				ClientId: id,
			},
		},
	}

	go func() {
		for e := range c.events {
			ch <- e
		}
	}()

	return ch, nil
}

// CallChatClient ...
func (s *Frontend) CallChatClient(ctx context.Context, r *pb.ChatClientCallRequest) error {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return ErrAuthenticationRequired
	}

	clientIf, _ := session.Values.Load(r.ClientId)
	client, ok := clientIf.(*ChatThing)
	if !ok {
		return errors.New("client id does not exist")
	}

	switch b := r.Body.(type) {
	case *pb.ChatClientCallRequest_Message_:
		client.SendMessage(b.Message)
	case *pb.ChatClientCallRequest_RunClient_:
		client.RunClient()
	case *pb.ChatClientCallRequest_RunServer_:
		go client.RunServer()
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
		Key:         key.Private,
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
		invBytes, err = base64.StdEncoding.DecodeString(r.GetInvitationB64())
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

	var invitation *pb.InvitationV0
	err = proto.Unmarshal(wrapper.Data, invitation)
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

	membership, err := dao.NewNetworkMembershipFromInvite(invitation, inviteCSR)
	if err != nil {
		return nil, err
	}

	err = s.SaveNetworkMembership(ctx, membership)
	if err != nil {
		return nil, err
	}

	return &pb.CreateNetworkMembershipFromInvitationResponse{
		Membership: membership,
	}, nil
}

// SaveNetworkMembership saves a networks membership or returns an error if the user is already has a valid membership for that network
func (s *Frontend) SaveNetworkMembership(ctx context.Context, newMembership *pb.NetworkMembership) error {
	session := rpc.ContextSession(ctx)
	if session.Anonymous() {
		return ErrAuthenticationRequired
	}

	memberships, err := session.Store().GetNetworkMemberships()
	if err != nil {
		return err
	}

	for _, m := range memberships {
		// naïve approach, should we compare ca certs instead?
		if m.Name == newMembership.Name && !dao.CertIsExpired(m.Certificate) {
			return ErrAlreadyJoinedNetwork
		}
	}

	return session.Store().InsertNetworkMembership(newMembership)
}
