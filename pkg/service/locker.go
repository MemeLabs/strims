package service

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// NewLocker ...
func NewLocker(ctx context.Context, svc *NetworkServices) *Locker {
	svc.Network.Done()

	return &Locker{svc}
}

// Locker ...
type Locker struct {
	svc *NetworkServices
}

// NewMutex ...
func (m *Locker) NewMutex(key *pb.Key, salt []byte) *Mutex {
	return &Mutex{}
}

// Mutex ...
type Mutex struct {
	ctx  context.Context
	key  *pb.Key
	salt []byte
	svc  *NetworkServices
}

// Lock ...
func (m *Mutex) Lock(ctx context.Context) error {
	// b, err := proto.Marshal(&pb.PubSubEvent{
	// 	Body: &pb.PubSubEvent_Message_{
	// 		Message: &pb.PubSubEvent_Message{
	// 			Time: time.Now().UnixNano(),
	// 			Key:  m.key,
	// 			Body: m.body,
	// 		},
	// 	},
	// })
	// if err != nil {
	// 	return err
	// }

	// // addr := c.addr.Load().(*hostAddr)
	// var addr *hostAddr
	// return m.svc.Network.Send(addr.HostID, addr.Port, c.port, b)

	// ctx, _ = context.WithTimeout(ctx, 5*time.Second)
	// values, err := m.svc.HashTable.Get(ctx, m.key.Public, m.salt)
	// if err != nil {
	// 	return err
	// }

	// for v := range values {
	// 	v.Timestamp

	// }
	return nil
}

// Unlock ...
func (m *Mutex) Unlock() {

}
