package ppspp

import (
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestPeerChannelSchedulerStreamSubImplicitRequests(t *testing.T) {
	id, _ := DecodeSwarmID("ewOeQgqCCXYwVmR-nZIcbLfDszuIgV8l0Xj0OVa5Vw4")
	swarm, _ := NewSwarm(NewSwarmID(id), SwarmOptions{StreamCount: 8})

	logger, _ := zap.NewDevelopment()
	swarmScheduler := newPeerSwarmScheduler(logger, swarm)
	channelScheduler := swarmScheduler.ChannelScheduler(&mockPeerThing{}, &mockChannelWriterThing{}).(*peerChannelScheduler)

	peerHaveBin := binmap.NewBin(6, 0)
	requestBin := binmap.NewBin(5, 0)
	haveBin := binmap.NewBin(5, 0)

	stream := codec.Stream(0)
	startBin := requestBin.BaseRight().LayerRight()

	channelScheduler.peerHaveBins.Set(peerHaveBin)
	swarmScheduler.requestBins.Set(requestBin)
	swarmScheduler.haveBins.Set(haveBin)
	swarmScheduler.peerMaxHaveBin = peerHaveBin.BaseRight()
	swarmScheduler.haveBinMax = haveBin.BaseRight()

	swarmScheduler.doStreamSub(channelScheduler, stream, requestBin.BaseRight().LayerRight())

	assert.Equal(t, startBin, channelScheduler.requestStreams[stream], "expected channel scheduler start stream to be set after sub")
	assert.Equal(t, []binmap.Bin{31, 64, 80, 96, 112}, swarmScheduler.requestBins.IterateFilled().ToSlice(), "expected requested bins to be filled for stream bins after sub")

	swarmScheduler.Consume(store.Chunk{Bin: 64})
	channelScheduler.HandleData(64, time.Time{}, true)

	channelScheduler.addStreamCancel(stream)

	assert.Equal(t, []binmap.Bin{31, 64}, swarmScheduler.requestBins.IterateFilled().ToSlice(), "expected unreceived bins to be filled for stream bins after unsub")
}

func TestPeerChannelSchedulerFoo(t *testing.T) {
	id, _ := DecodeSwarmID("ewOeQgqCCXYwVmR-nZIcbLfDszuIgV8l0Xj0OVa5Vw4")
	swarm, _ := NewSwarm(NewSwarmID(id), SwarmOptions{StreamCount: 8})

	logger, _ := zap.NewDevelopment()
	swarmScheduler := newPeerSwarmScheduler(logger, swarm)

	const liveWindow = 16 * 1024
	const mtu = 16 * 1024

	var enqueueCalled, writeRequestCalled bool
	var writeRequestBin binmap.Bin

	p := &mockPeerThing{
		enqueueFunc: func(t *PeerWriterQueueTicket, w PeerWriter) {
			enqueueCalled = true
		},
	}
	w := &mockChannelWriterThing{
		cap: mtu,
		WriteRequestFunc: func(m codec.Request) error {
			writeRequestCalled = true
			writeRequestBin = m.Address.Bin()
			return nil
		},
	}
	channelScheduler := swarmScheduler.ChannelScheduler(p, w).(*peerChannelScheduler)
	channelScheduler.HandleHandshake(liveWindow)

	haveBin := binmap.NewBin(6, 0)

	err := channelScheduler.HandleHave(haveBin)
	assert.NoError(t, err)

	err = channelScheduler.HandleMessageEnd()
	assert.NoError(t, err)
	assert.True(t, enqueueCalled)

	_, err = channelScheduler.Write(mtu)
	assert.NoError(t, err)
	assert.True(t, writeRequestCalled)
	assert.True(t, haveBin.Contains(writeRequestBin))
}
