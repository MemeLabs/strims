package vpn

import (
	"context"
	"errors"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	vpnv1 "github.com/MemeLabs/strims/pkg/apis/vpn/v1"
	"github.com/MemeLabs/strims/pkg/debug"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var linkTimeout = 20 * time.Second

type exchange struct {
	id         uint64
	ctx        context.Context
	cancel     context.CancelFunc
	candidates *vnic.LinkCandidatePool
}

type PeerExchange interface {
	HandleMessage(msg *Message) error
	Connect(hostID kademlia.ID) (err error)
}

func newPeerExchange(logger *zap.Logger, network *Network) PeerExchange {
	return &peerExchange{
		logger:  logger,
		network: network,
	}
}

type peerExchange struct {
	logger    *zap.Logger
	network   *Network
	exchanges syncutil.Map[kademlia.ID, *exchange]
}

// HandleMessage ...
func (s *peerExchange) HandleMessage(msg *Message) error {
	var m vpnv1.PeerExchangeMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return err
	}

	switch b := m.Body.(type) {
	case *vpnv1.PeerExchangeMessage_LinkOffer_:
		go s.handleLinkOffer(b.LinkOffer, msg)
	case *vpnv1.PeerExchangeMessage_LinkAnswer_:
		go s.handleLinkAnswer(b.LinkAnswer, msg)
	default:
		return errors.New("unexpected message type")
	}
	return nil
}

func (s *peerExchange) mediatorCount() int {
	return s.exchanges.Len()
}

func (s *peerExchange) allowNewPeer() bool {
	return s.network.VNIC().PeerCount()+s.mediatorCount() < s.network.VNIC().MaxPeers()
}

func (s *peerExchange) handleLinkOffer(offer *vpnv1.PeerExchangeMessage_LinkOffer, msg *Message) {
	s.logger.Debug("handleLinkOffer")
	debug.PrintJSON(offer)

	logger := s.logger.With(zap.Stringer("host", msg.SrcHostID()))
	ctx, cancel := context.WithTimeout(context.Background(), linkTimeout)

	var err error
	defer func() {
		if err != nil {
			logger.Warn("handling link offer failed", zap.Error(err))
			s.sendAnswer(msg.SrcHostID(), offer.ExchangeId, nil, err)
			cancel()
		}
	}()

	if s.network.VNIC().HasPeer(msg.SrcHostID()) {
		err = errors.New("peer link already open")
		return
	}
	if !s.allowNewPeer() {
		err = errors.New("unable to add new peers")
		return
	}

	candidates, err := s.network.host.LinkCandidates(ctx)
	if err != nil {
		return
	}

	connected, err := candidates.SetRemoteDescriptions(offer.Descriptions)
	if err != nil {
		return
	}
	if connected {
		s.sendAnswer(msg.SrcHostID(), offer.ExchangeId, nil, nil)
		return
	}

	descriptions, err := candidates.LocalDescriptions()
	if err != nil {
		return
	}

	if err := s.sendAnswer(msg.SrcHostID(), offer.ExchangeId, descriptions, nil); err != nil {
		return
	}
}

func (s *peerExchange) handleLinkAnswer(answer *vpnv1.PeerExchangeMessage_LinkAnswer, msg *Message) {
	s.logger.Debug("handleLinkAnswer")
	debug.PrintJSON(answer)

	x, ok := s.exchanges.GetAndDelete(msg.SrcHostID())
	if !ok {
		return
	}
	if x.id != answer.ExchangeId {
		return
	}

	if len(answer.Descriptions) == 0 {
		s.logger.Debug(
			"answer contains no link candidate descriptions",
			zap.Stringer("host", msg.SrcHostID()),
			zap.String("error", answer.ErrorMessage),
		)
		return
	}

	_, err := x.candidates.SetRemoteDescriptions(answer.Descriptions)
	if err != nil {
		return
	}
}

func (s *peerExchange) Connect(hostID kademlia.ID) (err error) {
	if s.network.VNIC().HasPeer(hostID) || s.network.VNIC().ID().Equals(hostID) {
		return
	}

	x := &exchange{}
	if _, ok := s.exchanges.GetOrInsert(hostID, x); ok {
		return
	}

	x.id, err = dao.GenerateSnowflake()
	if err != nil {
		s.exchanges.Delete(hostID)
		return
	}

	x.ctx, x.cancel = context.WithTimeout(context.Background(), linkTimeout)
	x.candidates, err = s.network.host.LinkCandidates(x.ctx)
	if err != nil {
		s.exchanges.Delete(hostID)
		return
	}

	descriptions, err := x.candidates.LocalDescriptions()
	if err != nil {
		s.exchanges.Delete(hostID)
		return
	}

	return s.sendOffer(hostID, x.id, descriptions)
}

func (s *peerExchange) sendOffer(hostID kademlia.ID, xid uint64, descriptions []*vnicv1.LinkDescription) error {
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_LinkOffer_{
			LinkOffer: &vpnv1.PeerExchangeMessage_LinkOffer{
				ExchangeId:   xid,
				Descriptions: descriptions,
			},
		},
	}
	s.logger.Debug("sendOffer")
	debug.PrintJSON(msg)
	return s.network.SendProto(hostID, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}

func (s *peerExchange) sendAnswer(hostID kademlia.ID, xid uint64, descriptions []*vnicv1.LinkDescription, err error) error {
	answer := &vpnv1.PeerExchangeMessage_LinkAnswer{
		ExchangeId:   xid,
		Descriptions: descriptions,
	}
	if err != nil {
		answer.ErrorMessage = err.Error()
	}
	msg := &vpnv1.PeerExchangeMessage{
		Body: &vpnv1.PeerExchangeMessage_LinkAnswer_{
			LinkAnswer: answer,
		},
	}
	s.logger.Debug("sendAnswer")
	debug.PrintJSON(msg)
	return s.network.SendProto(hostID, vnic.PeerExchangePort, vnic.PeerExchangePort, msg)
}
