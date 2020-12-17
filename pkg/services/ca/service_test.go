package ca

import (
	"context"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/dao/daotest"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/services/servicestest"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func TestCA(t *testing.T) {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)

	cluster, err := servicestest.NewCluster(logger)
	assert.Nil(t, err)

	ctx := context.Background()

	_, err = NewServer(ctx, logger, cluster.Nodes[0].Client, cluster.Network)
	assert.Nil(t, err)

	time.Sleep(time.Second)

	invitation, err := dao.NewInvitationV0(cluster.Nodes[0].Profile.Key, cluster.Network.Certificate)
	assert.Nil(t, err)

	network, err := dao.NewNetworkFromInvitationV0(&daotest.IDGenerator{}, invitation, cluster.Nodes[1].Profile)
	assert.Nil(t, err)

	csr, err := dao.NewCertificateRequest(
		cluster.Nodes[1].Profile.Key,
		pb.KeyUsage_KEY_USAGE_PEER|pb.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject(cluster.Nodes[1].Profile.Name),
	)

	caClient, err := NewClient(logger, cluster.Nodes[1].Client)
	assert.Nil(t, err)

	req := &pb.CARenewRequest{
		Certificate:        network.Certificate,
		CertificateRequest: csr,
	}
	res := &pb.CARenewResponse{}
	err = caClient.Renew(ctx, req, res)
	assert.Nil(t, err)
}
