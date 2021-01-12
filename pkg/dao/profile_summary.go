package dao

import (
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
)

const profileSummaryKeyPrefix = "profileSummary:"

func prefixProfileSummaryKey(name string) string {
	return profileSummaryKeyPrefix + name
}

// GetProfileSummaries ...
func GetProfileSummaries(s kv.BlobStore) ([]*profilev1.ProfileSummary, error) {
	var profileBufs [][]byte
	err := s.View(metadataTable, func(tx kv.BlobTx) (err error) {
		profileBufs, err = tx.ScanPrefix(profileSummaryKeyPrefix)
		return err
	})
	if err != nil {
		return nil, err
	}

	profiles, err := appendUnmarshalled([]*profilev1.ProfileSummary{}, profileBufs...)
	if err != nil {
		return nil, err
	}

	return profiles.([]*profilev1.ProfileSummary), nil
}

// NewProfileSummary ...
func NewProfileSummary(p *profilev1.Profile) *profilev1.ProfileSummary {
	return &profilev1.ProfileSummary{
		Id:   p.Id,
		Name: p.Name,
	}
}
