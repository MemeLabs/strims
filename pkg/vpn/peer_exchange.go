// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vpn

import (
	"errors"
	"fmt"
	"sync"
	"sync/atomic"
	"time"

	vpnv1 "github.com/MemeLabs/strims/pkg/apis/vpn/v1"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/randutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var closedChan = make(chan struct{})

func init() {
	close(closedChan)
}

func newWebRTCMediator(hostID kademlia.ID, network *Network) *webRTCMediator {
	return &webRTCMediator{
		mediationID:           errutil.Must(randutil.Uint64()),
		id:                    hostID,
		network:               network,
		init:                  true,
		done:                  make(chan struct{}),
		remoteICECandiates:    make(chan []byte, 64),
		remoteDescriptionDone: make(chan struct{}),
	}
}

func newWebRTCMediatorFromOffer(
	hostID kademlia.ID,
	network *Network,
	remoteMediationID uint64,
	offer []byte,
) *webRTCMediator {
	return &webRTCMediator{
		mediationID:           errutil.Must(randutil.Uint64()),
		remoteMediationID:     remoteMediationID,
		id:                    hostID,
		network:               network,
		init:                  false,
		done:                  make(chan struct{}),
		remoteICECandiates:    make(chan []byte, 64),
		remoteDescriptionDone: closedChan,
		remoteDescription:     offer,
	}
}

type webRTCMediator struct {
	init             bool
	id               kademlia.ID
	network          *Network
	mediationID      uint64
	nextICESendIndex uint64

	lock                  sync.Mutex
	remoteMediationID     uint64
	closeErr              error
	remoteDescriptionDone chan struct{}
	remoteDescription     []byte

	remoteICELock        sync.Mutex
	remoteICEReadIndices uint64
	remoteICEAllIndices  uint64
	remoteICECandiates   chan []byte
	remoteICECloseOnce   sync.Once

	closeOnce sync.Once
	done      chan struct{}
}

func (m *webRTCMediator) Scheme() string {
	return vnic.WebRTCScheme
}

func (m *webRTCMediator) String() string {
	return fmt.Sprintf("%s:%x/%s", vnic.WebRTCScheme, m.network.Key(), m.id)
}

func (m *webRTCMediator) HasOffer() bool {
	m.lock.Lock()
	defer m.lock.Unlock()
	return m.remoteDescription == nil
}

func (m *webRTCMediator) SetAnswer(remoteMediationID uint64, answer []byte) error {
	m.lock.Lock()
	defer m.lock.Unlock()

	if remoteMediationID == 0 {
		return errors.New("remote mediation id must be non zero")
	}
	if answer == nil {
		return errors.New("remote description empty")
	}

	if m.remoteDescription != nil {
		return nil
	}

	m.remoteMediationID = remoteMediationID
	m.remoteDescription = answer
	close(m.remoteDescriptionDone)
	return nil
}

func (m *webRTCMediator) close(err error) {
	m.closeOnce.Do(func() {
		m.lock.Lock()
		m.closeErr = err
		m.lock.Unlock()

		close(m.done)
	})
}

func (m *webRTCMediator) addICECandidate(index uint64, candidate []byte) (bool, error) {
	if index > 64 {
		return false, errors.New("ice candidate index out of range")
	}

	m.remoteICELock.Lock()
	defer m.remoteICELock.Unlock()

	mask := uint64(1) << index
	if candidate == nil {
		m.remoteICEAllIndices = mask - 1
	} else if m.remoteICEReadIndices&mask == 0 {
		m.remoteICEReadIndices |= mask

		select {
		case m.remoteICECandiates <- candidate:
		default:
		}
	}

	if m.remoteICEAllIndices != 0 && m.remoteICEReadIndices == m.remoteICEAllIndices {
		m.remoteICECloseOnce.Do(func() { close(m.remoteICECandiates) })
		return true, nil
	}
	return false, nil
}

func (m *webRTCMediator) getRemoteDescription() ([]byte, error) {
	select {
	case <-m.remoteDescriptionDone:
		m.lock.Lock()
		defer m.lock.Unlock()
		return m.remoteDescription, nil
	case <-m.done:
		m.lock.Lock()
		defer m.lock.Unlock()
		return nil, fmt.Errorf("mediator closed: %w", m.closeErr)
	}
}

func (m *webRTCMediator) GetOffer() ([]byte, error) {
	if m.init {
		return nil, nil
	}

	return m.getRemoteDescription()
}

func (m *webRTCMediator) GetAnswer() ([]byte, error) {
	return m.getRemoteDescription()
}

func (m *webRTCMediator) GetICECandidates() <-chan []byte {
	return m.remoteICECandiates
}

func (m *webRTCMediator) SendOffer(offer []byte) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_MediationOffer_{
			MediationOffer: &vpnv1.PeerExchangeMessage_MediationOffer{
				MediationId: m.mediationID,
				Data:        offer,
			},
		},
	}
	return m.network.SendProto(m.id, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (m *webRTCMediator) SendAnswer(answer []byte) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_MediationAnswer_{
			MediationAnswer: &vpnv1.PeerExchangeMessage_MediationAnswer{
				MediationId: m.mediationID,
				Data:        answer,
			},
		},
	}
	return m.network.SendProto(m.id, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (m *webRTCMediator) SendICECandidate(candidate []byte) error {
	index := atomic.AddUint64(&m.nextICESendIndex, 1) - 1

	if _, err := m.getRemoteDescription(); err != nil {
		return err
	}

	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_MediationIceCandidate_{
			MediationIceCandidate: &vpnv1.PeerExchangeMessage_MediationIceCandidate{
				MediationId: m.mediationID,
				Index:       index,
				Data:        candidate,
			},
		},
	}
	return m.network.SendProto(m.id, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

// PeerExchange ...
type PeerExchange interface {
	Connect(hostID kademlia.ID) error
}

func newPeerExchange(logger *zap.Logger, network *Network) *peerExchange {
	return &peerExchange{
		logger:    logger,
		network:   network,
		mediators: map[kademlia.ID]*webRTCMediator{},
	}
}

// peerExchange ...
type peerExchange struct {
	logger        *zap.Logger
	network       *Network
	mediatorsLock sync.Mutex
	mediators     map[kademlia.ID]*webRTCMediator
}

// HandleMessage ...
func (s *peerExchange) HandleMessage(msg *Message) error {
	var m vpnv1.PeerExchangeMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return err
	}

	switch b := m.Body.(type) {
	case *vpnv1.PeerExchangeMessage_MediationOffer_:
		return s.handleMediationOffer(b.MediationOffer, msg)
	case *vpnv1.PeerExchangeMessage_MediationAnswer_:
		return s.handleMediationAnswer(b.MediationAnswer, msg)
	case *vpnv1.PeerExchangeMessage_MediationIceCandidate_:
		return s.handleMediationICECandidate(b.MediationIceCandidate, msg)
	case *vpnv1.PeerExchangeMessage_CallbackRequest_:
		return s.handleCallbackRequest(b.CallbackRequest, msg)
	case *vpnv1.PeerExchangeMessage_Rejection_:
		return s.handleRejection(b.Rejection, msg)
	default:
		return errors.New("unexpected message type")
	}
}

func (s *peerExchange) mediatorCount() int {
	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()
	return len(s.mediators)
}

func (s *peerExchange) allowNewPeer() bool {
	return s.network.VNIC().PeerCount()+s.mediatorCount() < s.network.VNIC().MaxPeers()
}

// Connect create mediator to negotiate connection with peer
func (s *peerExchange) Connect(hostID kademlia.ID) error {
	if s.network.VNIC().HasPeer(hostID) || s.network.VNIC().ID().Equals(hostID) {
		return nil
	}

	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()

	if _, ok := s.mediators[hostID]; ok {
		return nil
	}

	go func() {
		if err := s.dial(newWebRTCMediator(hostID, s.network)); err != nil {
			s.logger.Debug(
				"dial failed requesting callback",
				zap.Stringer("host", hostID),
				zap.Error(err),
			)
			if err := s.sendCallbackRequest(hostID); err != nil {
				s.logger.Debug(
					"send callback request failed",
					zap.Error(err),
				)
			}
		}
	}()

	return nil
}

func (s *peerExchange) sendCallbackRequest(hostID kademlia.ID) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_CallbackRequest_{
			CallbackRequest: &vpnv1.PeerExchangeMessage_CallbackRequest{},
		},
	}
	return s.network.SendProto(hostID, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (s *peerExchange) handleCallbackRequest(m *vpnv1.PeerExchangeMessage_CallbackRequest, msg *Message) error {
	if !s.allowNewPeer() {
		return nil
	}

	go func() {
		if err := s.dial(newWebRTCMediator(msg.SrcHostID(), s.network)); err != nil {
			s.logger.Debug(
				"dial failed handling callback request",
				zap.Error(err),
			)
		}
	}()
	return nil
}

func (s *peerExchange) sendRejection(hostID kademlia.ID, mediationID uint64) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_Rejection_{
			Rejection: &vpnv1.PeerExchangeMessage_Rejection{
				MediationId: mediationID,
			},
		},
	}
	return s.network.SendProto(hostID, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (s *peerExchange) handleRejection(m *vpnv1.PeerExchangeMessage_Rejection, msg *Message) error {
	s.mediatorsLock.Lock()
	t, ok := s.mediators[msg.SrcHostID()]
	s.mediatorsLock.Unlock()
	if !ok || t.mediationID != m.MediationId {
		return nil
	}

	s.logger.Debug(
		"cancelling rejected connection",
		zap.Stringer("host", t.id),
		zap.Uint64("mediationId", t.mediationID),
	)
	t.close(errors.New("remote host rejected offer"))
	s.mediatorsLock.Lock()
	delete(s.mediators, msg.SrcHostID())
	s.mediatorsLock.Unlock()

	return nil
}

func (s *peerExchange) handleMediationOffer(m *vpnv1.PeerExchangeMessage_MediationOffer, msg *Message) error {
	go func() {
		logger := s.logger.With(zap.Stringer("host", msg.SrcHostID()))

		if !s.allowNewPeer() {
			logger.Debug("rejecting offer")
			s.sendRejection(msg.SrcHostID(), m.MediationId)
			return
		}

		logger.Debug("handling offer")
		if err := s.dial(newWebRTCMediatorFromOffer(msg.SrcHostID(), s.network, m.MediationId, m.Data)); err != nil {
			logger.Debug(
				"dial failed for newMediatorFromOffer",
				zap.Error(err),
			)
		}
	}()
	return nil
}

func (s *peerExchange) dial(t *webRTCMediator) error {
	if s.network.VNIC().HasPeer(t.id) {
		return errors.New("duplicate connection: existing peer link found")
	}

	s.mediatorsLock.Lock()
	if pt, ok := s.mediators[t.id]; ok {
		if pt.mediationID < t.remoteMediationID || !t.HasOffer() {
			s.mediatorsLock.Unlock()
			return errors.New("duplicate connection: mediation in progress")
		}
		pt.close(errors.New("duplicate connection: mediator replaced"))
	}
	s.mediators[t.id] = t
	s.mediatorsLock.Unlock()

	s.logger.Debug(
		"creating connection",
		zap.Stringer("host", t.id),
		zap.Uint64("mediationId", t.mediationID),
	)

	errs := make(chan error, 1)
	go func() {
		errs <- s.network.host.Dial(t)

		s.mediatorsLock.Lock()
		delete(s.mediators, t.id)
		s.mediatorsLock.Unlock()
	}()

	var err error
	select {
	case err = <-errs:
	case <-time.After(20 * time.Second):
		err = errors.New("timeout")
	case <-t.done:
	}

	t.close(err)
	return err
}

func (s *peerExchange) handleMediationAnswer(m *vpnv1.PeerExchangeMessage_MediationAnswer, msg *Message) error {
	s.mediatorsLock.Lock()
	t, ok := s.mediators[msg.SrcHostID()]
	s.mediatorsLock.Unlock()
	if !ok {
		return fmt.Errorf("no mediator to handle answer from %s", msg.SrcHostID())
	}

	return t.SetAnswer(m.MediationId, m.Data)
}

func (s *peerExchange) handleMediationICECandidate(m *vpnv1.PeerExchangeMessage_MediationIceCandidate, msg *Message) error {
	s.mediatorsLock.Lock()
	t, ok := s.mediators[msg.SrcHostID()]
	s.mediatorsLock.Unlock()
	if !ok || t.remoteMediationID != m.MediationId {
		return fmt.Errorf("no mediator to handle ice candidate from %s", msg.SrcHostID())
	}

	done, err := t.addICECandidate(m.Index, m.Data)
	if err != nil {
		return err
	}
	if done {
		s.mediatorsLock.Lock()
		delete(s.mediators, msg.SrcHostID())
		s.mediatorsLock.Unlock()
	}
	return nil
}
