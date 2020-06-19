package dao

import "github.com/MemeLabs/go-ppspp/pkg/pb"

const profileSummaryKeyPrefix = "profileSummary:"

func prefixProfileSummaryKey(name string) string {
	return profileSummaryKeyPrefix + name
}

// GetProfileSummaries ...
func GetProfileSummaries(s BlobStore) ([]*pb.ProfileSummary, error) {
	var profileBufs [][]byte
	err := s.View(metadataTable, func(tx BlobTx) (err error) {
		profileBufs, err = tx.ScanPrefix(profileSummaryKeyPrefix)
		return err
	})
	if err != nil {
		return nil, err
	}

	profiles, err := appendUnmarshalled([]*pb.ProfileSummary{}, profileBufs...)
	if err != nil {
		return nil, err
	}

	return profiles.([]*pb.ProfileSummary), nil
}

// NewProfileSummary ...
func NewProfileSummary(p *pb.Profile) *pb.ProfileSummary {
	return &pb.ProfileSummary{
		Id:   p.Id,
		Name: p.Name,
	}
}
