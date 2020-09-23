package vpn

import (
	"crypto/rand"
	"encoding/binary"
	"errors"
	"fmt"
	"log"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// WebRTCMediator ...
type WebRTCMediator interface {
	Scheme() string
	GetOffer() ([]byte, error)
	GetAnswer() ([]byte, error)
	GetICECandidates() <-chan []byte
	SendOffer([]byte) error
	SendAnswer([]byte) error
	SendICECandidate([]byte) error
}

var closedChan = make(chan struct{})

func init() {
	close(closedChan)
}

func newMediationID() uint64 {
	var b [8]byte
	_, err := rand.Read(b[:])
	if err != nil {
		log.Println(err)
	}
	return binary.LittleEndian.Uint64(b[:])
}

func newMediator(hostID kademlia.ID, network *Network) *mediator {
	return &mediator{
		mediationID:           newMediationID(),
		id:                    hostID,
		network:               network,
		init:                  true,
		done:                  make(chan struct{}),
		remoteICECandiates:    make(chan []byte, 64),
		remoteDescriptionDone: make(chan struct{}),
	}
}

func newMediatorFromOffer(
	hostID kademlia.ID,
	network *Network,
	remoteMediationID uint64,
	offer []byte,
) *mediator {
	return &mediator{
		mediationID:           newMediationID(),
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

type mediator struct {
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

func (m *mediator) Scheme() string {
	return "webrtc"
}

func (m *mediator) String() string {
	return fmt.Sprintf("webrtc:%x/%s", m.network.CAKey(), m.id)
}

func (m *mediator) SetAnswer(remoteMediationID uint64, answer []byte) error {
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

func (m *mediator) close(err error) {
	m.closeOnce.Do(func() {
		m.closeErr = err
		close(m.done)
	})
}

func (m *mediator) addICECandidate(index uint64, candidate []byte) (bool, error) {
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

func (m *mediator) getRemoteDescription() ([]byte, error) {
	select {
	case <-m.remoteDescriptionDone:
		return m.remoteDescription, nil
	case <-m.done:
		return nil, fmt.Errorf("mediator closed: %w", m.closeErr)
	}
}

func (m *mediator) GetOffer() ([]byte, error) {
	if m.init {
		return nil, nil
	}

	return m.getRemoteDescription()
}

func (m *mediator) GetAnswer() ([]byte, error) {
	return m.getRemoteDescription()
}

func (m *mediator) GetICECandidates() <-chan []byte {
	return m.remoteICECandiates
}

func (m *mediator) SendOffer(offer []byte) error {
	msg := &pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_Offer_{
			Offer: &pb.PeerExchangeMessage_Offer{
				MediationId: m.mediationID,
				Data:        offer,
			},
		},
	}
	return sendProto(m.network, m.id, PeerExchangePort, PeerExchangePort, msg)
}

func (m *mediator) SendAnswer(answer []byte) error {
	msg := &pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_Answer_{
			Answer: &pb.PeerExchangeMessage_Answer{
				MediationId: m.mediationID,
				Data:        answer,
			},
		},
	}
	return sendProto(m.network, m.id, PeerExchangePort, PeerExchangePort, msg)
}

func (m *mediator) SendICECandidate(candidate []byte) error {
	index := atomic.AddUint64(&m.nextICESendIndex, 1) - 1

	if _, err := m.getRemoteDescription(); err != nil {
		return err
	}

	msg := &pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_IceCandidate_{
			IceCandidate: &pb.PeerExchangeMessage_IceCandidate{
				MediationId: m.mediationID,
				Index:       index,
				Data:        candidate,
			},
		},
	}
	return sendProto(m.network, m.id, PeerExchangePort, PeerExchangePort, msg)
}

// NewPeerExchange ...
func NewPeerExchange(logger *zap.Logger, network *Network) *PeerExchange {
	return &PeerExchange{
		logger:    logger,
		network:   network,
		mediators: map[kademlia.ID]*mediator{},
	}
}

// PeerExchange ...
type PeerExchange struct {
	logger        *zap.Logger
	network       *Network
	mediatorsLock sync.Mutex
	mediators     map[kademlia.ID]*mediator
}

// HandleMessage ...
func (s *PeerExchange) HandleMessage(msg *Message) (bool, error) {
	if !msg.Header.DstID.Equals(s.network.host.ID()) || msg.Hops() == 0 {
		return true, nil
	}

	var m pb.PeerExchangeMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return false, err
	}

	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()

	switch b := m.Body.(type) {
	case *pb.PeerExchangeMessage_Offer_:
		return false, s.handleOffer(b.Offer, msg)
	case *pb.PeerExchangeMessage_Answer_:
		return false, s.handleAnswer(b.Answer, msg)
	case *pb.PeerExchangeMessage_IceCandidate_:
		return false, s.handleICECandidate(b.IceCandidate, msg)
	case *pb.PeerExchangeMessage_CallbackRequest_:
		return false, s.handleCallbackRequest(b.CallbackRequest, msg)
	}

	// return false, errors.New("unexpected message type")
	return false, nil
}

// Connect create mediator to negotiate connection with peer
func (s *PeerExchange) Connect(hostID kademlia.ID) error {
	// TODO: handle races
	if _, ok := s.network.host.GetPeer(hostID); ok {
		return nil
	}

	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()

	if _, ok := s.mediators[hostID]; ok {
		return nil
	}

	go func() {
		if err := s.dial(newMediator(hostID, s.network)); err != nil {
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

func (s *PeerExchange) sendCallbackRequest(hostID kademlia.ID) error {
	msg := &pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_CallbackRequest_{
			CallbackRequest: &pb.PeerExchangeMessage_CallbackRequest{},
		},
	}
	return sendProto(s.network, hostID, PeerExchangePort, PeerExchangePort, msg)
}

func (s *PeerExchange) handleCallbackRequest(m *pb.PeerExchangeMessage_CallbackRequest, msg *Message) error {
	go func() {
		if err := s.dial(newMediator(msg.FromHostID(), s.network)); err != nil {
			s.logger.Debug(
				"dial failed handling callback request",
				zap.Error(err),
			)
		}
	}()
	return nil
}

func (s *PeerExchange) handleOffer(m *pb.PeerExchangeMessage_Offer, msg *Message) error {
	s.logger.Debug(
		"handling offer",
		zap.Stringer("host", msg.FromHostID()),
	)
	go func() {
		if err := s.dial(newMediatorFromOffer(msg.FromHostID(), s.network, m.MediationId, m.Data)); err != nil {
			s.logger.Debug(
				"dial failed for newMediatorFromOffer",
				zap.Error(err),
			)
		}
	}()
	return nil
}

func (s *PeerExchange) dial(t *mediator) error {
	if _, ok := s.mediators[t.id]; ok {
		return errors.New("duplicate connection attempt")
	}

	s.logger.Debug(
		"creating connection",
		zap.Stringer("host", t.id),
	)

	s.mediatorsLock.Lock()
	s.mediators[t.id] = t
	s.mediatorsLock.Unlock()

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

func (s *PeerExchange) handleAnswer(m *pb.PeerExchangeMessage_Answer, msg *Message) error {
	t, ok := s.mediators[msg.FromHostID()]
	if !ok {
		return fmt.Errorf("no mediator to handle answer from %s", msg.FromHostID())
	}

	return t.SetAnswer(m.MediationId, m.Data)
}

func (s *PeerExchange) handleICECandidate(m *pb.PeerExchangeMessage_IceCandidate, msg *Message) error {
	t, ok := s.mediators[msg.FromHostID()]
	if !ok || t.remoteMediationID != m.MediationId {
		return fmt.Errorf("no mediator to handle ice candidate from %s", msg.FromHostID())
	}

	done, err := t.addICECandidate(m.Index, m.Data)
	if err != nil {
		return err
	}
	if done {
		delete(s.mediators, msg.FromHostID())
	}
	return nil
}
