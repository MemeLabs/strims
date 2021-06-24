package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func newTestPeerSwarmScheduler() *peerSwarmScheduler {
	id, _ := DecodeSwarmID("ewOeQgqCCXYwVmR-nZIcbLfDszuIgV8l0Xj0OVa5Vw4")
	swarm, _ := NewSwarm(NewSwarmID(id), SwarmOptions{StreamCount: 8})

	logger, _ := zap.NewDevelopment()
	return newPeerSwarmScheduler(logger, swarm)
}

func TestPeerSwarmSchedulerStartStopChannel(t *testing.T) {
	swarmScheduler := newTestPeerSwarmScheduler()

	var closeChannelCalled bool
	peer := &mockPeerThing{
		closeChannelFunc: func(w PeerWriter) {
			closeChannelCalled = true
		},
	}

	channelScheduler := swarmScheduler.ChannelScheduler(peer, &mockChannelWriterThing{}).(*peerChannelScheduler)
	assert.NotNil(t, channelScheduler)

	swarmScheduler.CloseChannel(peer)
	assert.True(t, closeChannelCalled, "expected peer closeChannel to be called")
}

func TestPeerChannelSchedulerStreamSubImplicitRequests(t *testing.T) {
	swarmScheduler := newTestPeerSwarmScheduler()
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
	channelScheduler.HandleData(64, timeutil.NilTime, true)

	channelScheduler.addStreamCancel(stream)

	assert.Equal(t, []binmap.Bin{31, 64}, swarmScheduler.requestBins.IterateFilled().ToSlice(), "expected unreceived bins to be filled for stream bins after unsub")
}

func TestPeerChannelSchedulerFoo(t *testing.T) {
	swarmScheduler := newTestPeerSwarmScheduler()

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

// type testFoo struct {
// 	streamLayer uint64
// 	streamBits  uint64
// 	streams     []peerSchedulerStreamReceivedChunks
// }

// func (s *testFoo) addReceivedChunk(b binmap.Bin) {
// 	o := b.BaseOffset()
// 	l := b.BaseLength()
// 	for i := uint64(0); i < l; i++ {
// 		log.Printf("stream %d off %d", o&s.streamBits, (o+i)>>s.streamLayer)
// 		s.streams[o&s.streamBits].addReceivedChunk((o + i) >> s.streamLayer)
// 	}
// }

// func TestThing2(t *testing.T) {
// 	streamCount := uint64(4)
// 	k := testFoo{
// 		streamLayer: uint64(bits.TrailingZeros64(streamCount)),
// 		streamBits:  streamCount - 1,
// 		streams:     make([]peerSchedulerStreamReceivedChunks, streamCount),
// 	}

// 	for i := binmap.Bin(0); i < 32; i += 8 {
// 		k.addReceivedChunk(i)
// 	}

// 	k.addReceivedChunk(18)
// 	k.addReceivedChunk(26)
// 	k.addReceivedChunk(2)
// 	k.addReceivedChunk(10)

// 	spew.Dump(k)
// }
