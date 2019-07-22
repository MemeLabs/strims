package encoding

type Swarm struct {
	UID      int64
	ID       *SwarmID
	channels *ChannelsMap
}

func (s *Swarm) handleMemeRequest(w MemeWriter, r *MemeRequest) {

}

func NewSwarm(uid int64, id *SwarmID) *Swarm {
	return &Swarm{
		UID:      uid,
		ID:       id,
		channels: NewChannelsMap(),
	}
}
