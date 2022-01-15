package dao

import (
	"encoding/binary"
	"errors"
	"strconv"

	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/mathutil"
	"google.golang.org/protobuf/proto"
)

const (
	certificateLogPrefix                  = "certificateLog:"
	certificateLogNetworkIndexPrefix      = "certificateLogNetwork:"
	certificateLogSubjectIndexPrefix      = "certificateLogSubject:"
	certificateLogSerialNumberIndexPrefix = "certificateLogSerialNumber:"
)

func prefixCertificateLogKey(id uint64) string {
	return certificateLogPrefix + strconv.FormatUint(id, 10)
}

func formatCertificateLogSubjectIndexKey(networkID uint64, subject string) []byte {
	return append(formatCertificateLogNetworkIndexKey(networkID), []byte(subject)...)
}

func formatCertificateLogSerialNumberIndexKey(networkID uint64, serialNumber []byte) []byte {
	return append(formatCertificateLogNetworkIndexKey(networkID), serialNumber...)
}

func formatCertificateLogNetworkIndexKey(networkID uint64) []byte {
	b := make([]byte, 8, 40)
	binary.BigEndian.PutUint64(b, networkID)
	return b
}

// NewCertificateLog ...
func NewCertificateLog(s IDGenerator, networkID uint64, cert *certificate.Certificate) (*networkv1ca.CertificateLog, error) {
	id, err := s.GenerateID()
	if err != nil {
		return nil, err
	}

	c := proto.Clone(cert).(*certificate.Certificate)
	if p := c.GetParent(); p != nil {
		c.ParentOneof = &certificate.Certificate_ParentSerialNumber{
			ParentSerialNumber: p.SerialNumber,
		}
	}

	return &networkv1ca.CertificateLog{
		Id:          id,
		NetworkID:   networkID,
		Certificate: c,
	}, nil
}

// InsertCertificateLog ...
func InsertCertificateLog(s kv.RWStore, v *networkv1ca.CertificateLog) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		prev, err := GetCertificateLogBySubject(tx, v.NetworkID, v.Certificate.Subject)
		if err != nil {
			if err != kv.ErrRecordNotFound {
				return err
			}
		} else {
			if prev.Certificate.NotBefore >= v.Certificate.NotBefore {
				return errors.New("newer certificate for subject found in ca log")
			}
		}

		err = SetSecondaryIndex(tx, certificateLogSubjectIndexPrefix, formatCertificateLogSubjectIndexKey(v.NetworkID, v.Certificate.Subject), v.Id)
		if err != nil {
			return err
		}
		err = SetUniqueSecondaryIndex(tx, certificateLogSerialNumberIndexPrefix, formatCertificateLogSerialNumberIndexKey(v.NetworkID, v.Certificate.SerialNumber), v.Id)
		if err != nil {
			return err
		}
		err = SetSecondaryIndex(tx, certificateLogNetworkIndexPrefix, formatCertificateLogNetworkIndexKey(v.NetworkID), v.Id)
		if err != nil {
			return err
		}
		return tx.Put(prefixCertificateLogKey(v.Id), v)
	})
}

// DeleteCertificateLog ...
func DeleteCertificateLog(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		v := &networkv1ca.CertificateLog{}
		err = tx.Get(prefixCertificateLogKey(id), v)
		if err != nil {
			return err
		}

		err = DeleteSecondaryIndex(tx, certificateLogSubjectIndexPrefix, formatCertificateLogSubjectIndexKey(v.NetworkID, v.Certificate.Subject), id)
		if err != nil {
			return err
		}
		err = DeleteSecondaryIndex(tx, certificateLogSerialNumberIndexPrefix, formatCertificateLogSerialNumberIndexKey(v.NetworkID, v.Certificate.SerialNumber), id)
		if err != nil {
			return err
		}
		err = DeleteSecondaryIndex(tx, certificateLogNetworkIndexPrefix, formatCertificateLogNetworkIndexKey(v.NetworkID), id)
		if err != nil {
			return err
		}
		return tx.Delete(prefixCertificateLogKey(id))
	})
}

// DeleteCertificateLogByNetwork ...
func DeleteCertificateLogByNetwork(s kv.RWStore, networkID uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		ids, err := ScanSecondaryIndex(tx, certificateLogNetworkIndexPrefix, formatCertificateLogNetworkIndexKey(networkID))
		if err != nil {
			return err
		}
		for _, id := range ids {
			if err := DeleteCertificateLog(tx, id); err != nil {
				return err
			}
		}
		return nil
	})
}

// GetCertificateLog ...
func GetCertificateLog(s kv.Store, id uint64) (v *networkv1ca.CertificateLog, err error) {
	v = &networkv1ca.CertificateLog{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixCertificateLogKey(id), v)
	})
	return
}

// GetCertificateLogBySubject ...
func GetCertificateLogBySubject(s kv.Store, networkID uint64, subject string) (v *networkv1ca.CertificateLog, err error) {
	v = &networkv1ca.CertificateLog{}
	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, certificateLogSubjectIndexPrefix, formatCertificateLogSubjectIndexKey(networkID, subject))
		if err != nil {
			return err
		}
		return tx.Get(prefixCertificateLogKey(mathutil.Max(ids...)), v)
	})
	return
}

// GetCertificateLogBySerialNumber ...
func GetCertificateLogBySerialNumber(s kv.Store, networkID uint64, serialNumber []byte) (v *networkv1ca.CertificateLog, err error) {
	v = &networkv1ca.CertificateLog{}
	err = s.View(func(tx kv.Tx) error {
		id, err := GetUniqueSecondaryIndex(tx, certificateLogSerialNumberIndexPrefix, formatCertificateLogSerialNumberIndexKey(networkID, serialNumber))
		if err != nil {
			return err
		}

		return tx.Get(prefixCertificateLogKey(id), v)
	})
	return
}

// GetCertificateLogs ...
func GetCertificateLogs(s kv.Store, networkID uint64) (v []*networkv1ca.CertificateLog, err error) {
	v = []*networkv1ca.CertificateLog{}
	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, certificateLogNetworkIndexPrefix, strconv.AppendUint(nil, networkID, 10))
		if err != nil {
			return err
		}

		for _, id := range ids {
			e, err := GetCertificateLog(tx, id)
			if err != nil {
				return err
			}
			v = append(v, e)
		}
		return nil
	})
	return
}
