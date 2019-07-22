package main

import (
	"github.com/MemeLabs/go-ppspp/pkg/iface"
	"github.com/davecgh/go-spew/spew"
	"github.com/gogo/protobuf/proto"
)

type Scheduler struct {

}

type Swarm struct {

}

type Peer struct {

}

type Client struct {
	scheduler *Scheduler
	swarms map[string]*Swarm
	peers map[string]*Peer
}

func main() {

}
