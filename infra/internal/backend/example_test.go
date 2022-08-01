package backend_test

import (
	"database/sql"
	"log"

	_ "github.com/lib/pq"
)

var (
	wgIPRangeStart = "10.0.0.1"

	db *sql.DB
)

func leaseWGIP(lesseeType string, lesseeID int64) (string, error) {
	query := `
	INSERT INTO wireguard_ip_leases (
		SELECT $1, $2, min(t.ip)
		FROM (
			SELECT $3 ip
			UNION
			SELECT ip + 1
			FROM wireguard_ip_leases
		) t
		LEFT JOIN wireguard_ip_leases
		ON (t.ip = wireguard_ip_leases.ip)
		WHERE wireguard_ip_leases.ip IS NULL
	)
	RETURNING ip;`

	tx, err := db.Begin()
	if err != nil {
		return "", err
	}
	var ip sql.NullString
	if err := tx.QueryRow(query, lesseeType, lesseeID, wgIPRangeStart).Scan(&ip); err != nil {
		if err := tx.Rollback(); err != nil {
			return "", err
		}
		return "", err
	}
	if err := tx.Commit(); err != nil {
		return "", err
	}

	return ip.String, nil
}

func releaseWGIPLease(lesseeType string, lesseeID int64) error {
	// delete from wireguard_ip_leases...
	return nil
}

func ExampleBackend_leaseWGIP() {
	nodeIP, err := leaseWGIP("node", 1234)
	log.Println(nodeIP, err)

	peerIP, err := leaseWGIP("external_peer", 1234)
	log.Println(peerIP, err)
}
