package network

import (
	"crypto/ed25519"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/golang/protobuf/ptypes"
	"github.com/petar/GoLLRB/llrb"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

func TestDuplicateNameCreateRequestFails(t *testing.T) {
	nickServ := getNickServ(t)

	record := &pb.NickservNick{
		Id:                       1,
		Nick:                     "wrxst",
		Key:                      []byte{0xDE, 0xAD},
		RemainingNameChangeQuota: 0,
	}

	_, err := nickServ.store.Insert(record)
	assert.NoError(t, err)

	record2 := &pb.NickservNick{
		Id:                       2,
		Nick:                     "wrxst",
		Key:                      []byte{0xBE, 0xEF},
		RemainingNameChangeQuota: 123,
	}
	record2, err = nickServ.store.Insert(record2)
	assert.EqualError(t, err, ErrNameAlreadyTaken.Error())
	assert.Nil(t, record2)
}

func TestAdminCanUpdateNicks(t *testing.T) {

}

func TestRequestFailsOnUnverifiedSignature(t *testing.T) {
	key := generateED25519Key(t)
	nickServ := getNickServ(t)
	m := &pb.NickServRPCCommand{
		SourcePublicKey: key.Public,
		Body: &pb.NickServRPCCommand_Update_{
			Update: &pb.NickServRPCCommand_Update{
				Param: &pb.NickServRPCCommand_Update_Nick{
					Nick: &pb.NickServRPCCommand_Update_ChangeNick{
						NewNick: "Versicarius",
						OldNick: "wrxst",
					},
				},
			},
		},
		RequestId: 123,
	}
	body, err := proto.Marshal(m)
	assert.NoError(t, err)

	message := &vpn.Message{
		Body:     body,
		Trailers: vpn.Trailers{vpn.MessageTrailer{Signature: []byte{0xAB}}},
	}

	_, err = nickServ.HandleMessage(message)
	assert.EqualError(t, err, dao.ErrInvalidSignature.Error())
}

func TestNameChangeQuota(t *testing.T) {
	nickServ := getNickServ(t)

	record := &pb.NickservNick{
		Id:                       1,
		Nick:                     "wrxst",
		Key:                      []byte{0xDE, 0xAD},
		RemainingNameChangeQuota: 0,
	}

	_, err := nickServ.store.Insert(record)
	assert.NoError(t, err)

	update := &pb.NickServRPCCommand_Update{
		Param: &pb.NickServRPCCommand_Update_Nick{
			Nick: &pb.NickServRPCCommand_Update_ChangeNick{
				NewNick: "Versicarius",
				OldNick: "wrxst",
			},
		},
	}

	resp, err := nickServ.handleUpdate([]byte{0xDE, 0xAD}, update)
	assert.EqualError(t, err, ErrNameChangesExhausted.Error())
	assert.Nil(t, resp)
}

func TestSignVerifyNickServToken(t *testing.T) {
	t.Skip("unimplemented")
	token := &pb.NickServToken{
		Key:        []byte{0xDE, 0xAD},
		Nick:       "foo",
		Roles:      []string{"mod", "weeb", "admin"},
		ValidUntil: ptypes.TimestampNow(),
	}

	key := generateED25519Key(t)
	err := signNickToken(token, key)
	assert.NoError(t, err)

	valid, err := VerifyNickToken(token)
	assert.NoError(t, err)
	assert.True(t, valid)

	// order of roles should not matter
	token.Roles = []string{"admin", "weeb", "mod"}
	valid, err = VerifyNickToken(token)
	assert.NoError(t, err)
	assert.True(t, valid)

	token.Nick = "bar"
	valid, err = VerifyNickToken(token)
	assert.NoError(t, err)
	assert.False(t, valid)
}

func TestInMemoryStore(t *testing.T) {
	store := createStore(t)
	key := []byte{0xBE, 0xEF}

	record := &pb.NickservNick{
		Id:   1,
		Nick: "bob",
		Key:  key,
	}

	// insert record
	record, err := store.Insert(record)
	assert.NoError(t, err)

	// ensure that record was inserted correctly
	r := store.nicks["bob"].record
	assert.NotNil(t, r)
	assert.True(t, proto.Equal(record, r))

	r2, err := store.Retrieve(key)
	assert.NoError(t, err)
	assert.True(t, proto.Equal(record, r2))

	newRecord := &pb.NickservNick{
		Id:   2,
		Nick: "brady",
		Key:  key,
	}

	// update record
	newRecord, err = store.Update(newRecord)
	assert.NoError(t, err)

	r3 := store.nicks["bob"]
	assert.Nil(t, r3)

	r4 := store.nicks["brady"].record
	assert.NotNil(t, r4)

	assert.True(t, proto.Equal(newRecord, r4))
}

func TestInvalidRole(t *testing.T) {
	nickServ := getNickServ(t)

	record := &pb.NickservNick{
		Id:   1,
		Nick: "bob",
		Key:  []byte{0xDE, 0xAD},
	}

	// insert record
	_, err := nickServ.store.Insert(record)
	assert.NoError(t, err)

	update := &pb.NickServRPCCommand_Update{
		Param: &pb.NickServRPCCommand_Update_Roles_{
			Roles: &pb.NickServRPCCommand_Update_Roles{
				Roles: []string{"mod", "weeb", "superadmin"},
			},
		},
	}

	resp, err := nickServ.handleUpdate([]byte{0xDE, 0xAD}, update)
	assert.EqualError(t, err, ErrRoleNotExist.Error())
	assert.Nil(t, resp)
}

func createStore(t *testing.T) *NickServStore {
	// TODO: is this the right way to get a kv store for testing?
	t.Helper()

	profile, err := dao.NewProfile("jbpratt")
	assert.Nil(t, err, "failed to create profile")

	key, err := dao.NewStorageKey("majoraautumn")
	assert.Nil(t, err, "failed to storage key")

	kvStore, err := memkv.NewStore("strims")
	assert.Nil(t, err, "failed to kv store")

	pfStore := dao.NewProfileStore(1, kvStore, key)
	assert.Nil(t, pfStore.Init(profile), "failed to create profile store")

	return &NickServStore{
		records: llrb.New(),
		nicks:   make(map[string]*nickServItem),
		kv:      pfStore,
	}
}

func getNickServ(t *testing.T) *NickServ {
	t.Helper()
	return &NickServ{
		logger:          zap.L().WithOptions(zap.Development()),
		nameChangeQuota: 2,
		tokenTTL:        time.Second * 5,
		store:           createStore(t),
		roles: map[string]struct{}{
			"mod":   {},
			"admin": {},
			"weeb":  {},
		},
	}
}

func generateED25519Key(t *testing.T) *pb.Key {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(nil)
	assert.Nil(t, err, "failed to generate ed25519 key")

	return &pb.Key{
		Type:    pb.KeyType_KEY_TYPE_ED25519,
		Public:  pub,
		Private: priv,
	}
}
