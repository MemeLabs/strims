package pathutil

import (
	"fmt"
	"os"
	"strings"
)

func Resolve(path string) (string, error) {
	if strings.HasPrefix(path, "~") {
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("unable to locate home directory: %w", err)
		}
		path = strings.Replace(path, "~", homeDir, 1)
	}
	return path, nil
}
