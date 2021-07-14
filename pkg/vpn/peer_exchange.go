package vpn

import (
	"errors"
	"fmt"
	"sync"
	"sync/atomic"
	"time"

	vpnv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vpn/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var closedChan = make(chan struct{})

func init() {
	close(closedChan)
}

func newWebRTCMediator(hostID kademlia.ID, network *Network) *webRTCMediator {
	return &webRTCMediator{
		mediationID:           randutil.MustUint64(),
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
		mediationID:           randutil.MustUint64(),
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
	mediationID           uint64
	remoteMediationID     uint64
	id                    kademlia.ID
	network               *Network
	init                  bool
	closeErr              error
	closeOnce             sync.Once
	done                  chan struct{}
	nextICESendIndex      uint64
	remoteICELock         sync.Mutex
	remoteICEReadIndices  uint64
	remoteICELastIndex    uint64
	remoteICECandiates    chan []byte
	remoteICECloseOnce    sync.Once
	remoteDescriptionDone chan struct{}
	remoteDescription     []byte
}

func (m *webRTCMediator) Scheme() string {
	return vnic.WebRTCScheme
}

func (m *webRTCMediator) String() string {
	return fmt.Sprintf("%s:%x/%s", vnic.WebRTCScheme, m.network.Key(), m.id)
}

func (m *webRTCMediator) SetAnswer(remoteMediationID uint64, answer []byte) error {
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
		m.closeErr = err
		close(m.done)
	})
}

func (m *webRTCMediator) addICECandidate(index uint64, candidate []byte) (bool, error) {
	if index > 64 {
		return false, errors.New("ice candidate index out of range")
	}

	m.remoteICELock.Lock()
	defer m.remoteICELock.Unlock()

	index = 1 << index
	if m.remoteICEReadIndices&index != 0 {
		return false, nil
	}
	if candidate == nil {
		m.remoteICELastIndex = index
	}
	m.remoteICEReadIndices |= index

	if candidate != nil {
		select {
		case m.remoteICECandiates <- candidate:
		default:
		}
	}

	if m.remoteICELastIndex != 0 && m.remoteICEReadIndices == m.remoteICELastIndex^(m.remoteICELastIndex-1) {
		m.remoteICECloseOnce.Do(func() { close(m.remoteICECandiates) })
		return true, nil
	}
	return false, nil
}

func (m *webRTCMediator) getRemoteDescription() ([]byte, error) {
	select {
	case <-m.remoteDescriptionDone:
		return m.remoteDescription, nil
	case <-m.done:
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
		Body: &vpnv1.PeerExchangeMessage_Offer_{
			Offer: &vpnv1.PeerExchangeMessage_Offer{
				MediationId: m.mediationID,
				Data:        offer,
			},
		},
	}
	return m.network.SendProto(m.id, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (m *webRTCMediator) SendAnswer(answer []byte) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_Answer_{
			Answer: &vpnv1.PeerExchangeMessage_Answer{
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
		Body: &vpnv1.PeerExchangeMessage_IceCandidate_{
			IceCandidate: &vpnv1.PeerExchangeMessage_IceCandidate{
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

	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()

	switch b := m.Body.(type) {
	case *vpnv1.PeerExchangeMessage_Offer_:
		return s.handleOffer(b.Offer, msg)
	case *vpnv1.PeerExchangeMessage_Answer_:
		return s.handleAnswer(b.Answer, msg)
	case *vpnv1.PeerExchangeMessage_IceCandidate_:
		return s.handleICECandidate(b.IceCandidate, msg)
	case *vpnv1.PeerExchangeMessage_CallbackRequest_:
		return s.handleCallbackRequest(b.CallbackRequest, msg)
	default:
		return errors.New("unexpected message type")
	}
}

// Connect create mediator to negotiate connection with peer
func (s *peerExchange) Connect(hostID kademlia.ID) error {
	if s.network.VNIC().HasPeer(hostID) {
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

func (s *peerExchange) handleOffer(m *vpnv1.PeerExchangeMessage_Offer, msg *Message) error {
	s.logger.Debug(
		"handling offer",
		zap.Stringer("host", msg.SrcHostID()),
	)
	go func() {
		if err := s.dial(newWebRTCMediatorFromOffer(msg.SrcHostID(), s.network, m.MediationId, m.Data)); err != nil {
			s.logger.Debug(
				"dial failed for newMediatorFromOffer",
				zap.Error(err),
			)
		}
	}()
	return nil
}

func (s *peerExchange) dial(t *webRTCMediator) error {
	if s.network.VNIC().HasPeer(t.id) {
		return errors.New("duplicate connection attempt")
	}

	s.mediatorsLock.Lock()
	if _, ok := s.mediators[t.id]; ok {
		s.mediatorsLock.Unlock()
		return errors.New("duplicate connection attempt")
	}
	s.mediators[t.id] = t
	s.mediatorsLock.Unlock()

	s.logger.Debug(
		"creating connection",
		zap.Stringer("host", t.id),
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

func (s *peerExchange) handleAnswer(m *vpnv1.PeerExchangeMessage_Answer, msg *Message) error {
	t, ok := s.mediators[msg.SrcHostID()]
	if !ok {
		return fmt.Errorf("no mediator to handle answer from %s", msg.SrcHostID())
	}

	return t.SetAnswer(m.MediationId, m.Data)
}

func (s *peerExchange) handleICECandidate(m *vpnv1.PeerExchangeMessage_IceCandidate, msg *Message) error {
	t, ok := s.mediators[msg.SrcHostID()]
	if !ok || t.remoteMediationID != m.MediationId {
		return fmt.Errorf("no mediator to handle ice candidate from %s", msg.SrcHostID())
	}

	done, err := t.addICECandidate(m.Index, m.Data)
	if err != nil {
		return err
	}
	if done {
		delete(s.mediators, msg.SrcHostID())
	}
	return nil
}
