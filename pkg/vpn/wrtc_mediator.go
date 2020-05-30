package vpn

import (
	"crypto/rand"
	"encoding/binary"
	"errors"
	"fmt"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
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
		panic(err)
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

func (m *mediator) close() {
	m.closeOnce.Do(func() { close(m.done) })
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
		return nil, errors.New("mediator closed")
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
	b, _ := proto.Marshal(&pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_Offer_{
			Offer: &pb.PeerExchangeMessage_Offer{
				MediationId: m.mediationID,
				Data:        offer,
			},
		},
	})
	return m.network.Send(m.id, PeerExchangePort, PeerExchangePort, b)
}

func (m *mediator) SendAnswer(answer []byte) error {
	b, _ := proto.Marshal(&pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_Answer_{
			Answer: &pb.PeerExchangeMessage_Answer{
				MediationId: m.mediationID,
				Data:        answer,
			},
		},
	})
	return m.network.Send(m.id, PeerExchangePort, PeerExchangePort, b)
}

func (m *mediator) SendICECandidate(candidate []byte) error {
	index := atomic.AddUint64(&m.nextICESendIndex, 1) - 1

	if _, err := m.getRemoteDescription(); err != nil {
		return err
	}

	b, _ := proto.Marshal(&pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_IceCandidate_{
			IceCandidate: &pb.PeerExchangeMessage_IceCandidate{
				MediationId: m.mediationID,
				Index:       index,
				Data:        candidate,
			},
		},
	})
	return m.network.Send(m.id, PeerExchangePort, PeerExchangePort, b)
}

// NewPeerExchange ...
func NewPeerExchange(network *Network) *PeerExchange {
	return &PeerExchange{
		network:   network,
		mediators: map[kademlia.ID]*mediator{},
	}
}

// PeerExchange ...
type PeerExchange struct {
	network       *Network
	mediatorsLock sync.Mutex
	mediators     map[kademlia.ID]*mediator
}

// HandleMessage ...
func (s *PeerExchange) HandleMessage(msg *Message) (bool, error) {
	if !msg.Header.DstID.Equals(s.network.host.ID()) {
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
	s.mediatorsLock.Lock()
	defer s.mediatorsLock.Unlock()

	if _, ok := s.mediators[hostID]; ok {
		return nil
	}

	go func() {
		if err := s.dial(newMediator(hostID, s.network)); err != nil {
			s.network.host.logger.Debug(
				"dial failed requesting callback",
				logutil.ByteHex("host", hostID.Bytes(nil)),
				zap.Error(err),
			)
			s.sendCallbackRequest(hostID)
		}
	}()

	return nil
}

func (s *PeerExchange) sendCallbackRequest(hostID kademlia.ID) error {
	b, _ := proto.Marshal(&pb.PeerExchangeMessage{
		Body: &pb.PeerExchangeMessage_CallbackRequest_{
			CallbackRequest: &pb.PeerExchangeMessage_CallbackRequest{},
		},
	})
	return s.network.Send(hostID, PeerExchangePort, PeerExchangePort, b)
}

func (s *PeerExchange) handleCallbackRequest(m *pb.PeerExchangeMessage_CallbackRequest, msg *Message) error {
	go s.dial(newMediator(msg.FromHostID(), s.network))
	return nil
}

func (s *PeerExchange) handleOffer(m *pb.PeerExchangeMessage_Offer, msg *Message) error {
	s.network.host.logger.Debug(
		"handling offer",
		logutil.ByteHex("host", msg.FromHostID().Bytes(nil)),
	)
	go s.dial(newMediatorFromOffer(msg.FromHostID(), s.network, m.MediationId, m.Data))
	return nil
}

func (s *PeerExchange) dial(t *mediator) error {
	if _, ok := s.mediators[t.id]; ok {
		return errors.New("duplicate connection attempt")
	}

	s.network.host.logger.Debug(
		"creating connection",
		logutil.ByteHex("host", t.id.Bytes(nil)),
	)

	s.mediators[t.id] = t
	defer func() {
		s.mediatorsLock.Lock()
		defer s.mediatorsLock.Unlock()

		t.close()
		delete(s.mediators, t.id)
	}()

	errs := make(chan error, 1)
	go func() { errs <- s.network.host.Dial(t) }()

	select {
	case err := <-errs:
		return err
	case <-time.After(20 * time.Second):
		return errors.New("timeout")
	case <-t.done:
		return nil
	}
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
