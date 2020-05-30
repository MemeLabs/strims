package service

import (
	"log"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
)

// NewDirectoryService ...
func NewDirectoryService(opt *NetworkServices, options *pb.Network) *DirectoryService {
	d := &DirectoryService{
		opt:     opt,
		name:    options.Name,
		key:     options.Key,
		network: opt.Network,
	}

	return d
}

// DirectoryService ...
type DirectoryService struct {
	opt     *NetworkServices
	name    string
	key     *pb.Key
	network *vpn.Network
	lock    sync.Mutex
}

// TODO: publish location/ecdh key to network
// TODO: create directory swarm

// Run ...
// func (s *DirectoryService) Run() {
// 	for range time.NewTicker(time.Second * 10).C {
// 		if err := s.advertise(); err != nil {
// 			log.Println(err)
// 		}
// 	}
// }

// TODO: network.SendRaw(toward kademlia.ID, []byte)?
// func (s *DirectoryService) advertise() error {

// 	a := &pb.ServiceAdvertisement{
// 		HostId: s.network.host.ID().Bytes(),
// 		Port:   12,
// 		Name:   "DirectoryService",
// 	}
// 	b, err := proto.Marshal(a)
// 	if err != nil {
// 		return err
// 	}

// 	id, _ := kademlia.UnmarshalID(s.key.Public)

// 	m := &Message{
// 		Header: MessageHeader{
// 			DstID:     id,
// 			DstPort:   11,
// 			Seq:    uint16(atomic.AddUint64(&s.network.seq, 1)),
// 			Length: uint16(len(b)),
// 		},
// 		Body: b,
// 	}

// 	b = frameBuffer(uint16(m.Size()))
// 	defer freeFrameBuffer(b)
// 	if _, err := m.Marshal(b, s.network.host); err != nil {
// 		return err
// 	}

// 	var conns [5]kademlia.Interface
// 	k := s.network.links.Closest(m.Header.DstID, conns[:])
// 	for _, ci := range conns[:k] {
// 		c := ci.(Connection)

// 		if _, err := c.ch.WriteFrame(b); err != nil {
// 			log.Println(err)
// 		}
// 	}
// 	return nil
// }

// HandleMessage ...
func (s *DirectoryService) HandleMessage(msg *vpn.Message) (bool, error) {
	log.Println("got message...")

	return true, nil
}
