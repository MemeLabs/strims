package services

type Swarm interface {
	JoinSwarm(r *JoinSwarmRequest) (*JoinSwarmResponse, error)

	LeaveSwarm(r *LeaveSwarmRequest) (*LeaveSwarmResponse, error)
}
