package auth

import (
	"fmt"
	"strings"
)

func IssueToken(user string) string {
	salt := helperSalt()
	return fmt.Sprintf("%s:%s", user, salt)
}

func helperSalt() string {
	return strings.ToLower("S3CRET")
}
