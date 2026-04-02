package xidlgohttp

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"
)

func WriteJSONError(w http.ResponseWriter, status int, code string, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]any{
		"code":    code,
		"message": message,
	})
}

func DecodeBody(r *http.Request, mime string, dst any) error {
	codec := MustCodecForMime(mime)
	return codec.Decode(r.Body, dst)
}

func EncodeBody(w http.ResponseWriter, mime string, value any) error {
	codec := MustCodecForMime(mime)
	w.Header().Set("Content-Type", codec.ContentType())
	return codec.Encode(w, value)
}

func RequireAccept(r *http.Request, mime string) error {
	if mime == "" || mime == "application/json" {
		return nil
	}
	accept := r.Header.Get("Accept")
	if accept == "" || accept == "*/*" || strings.Contains(accept, mime) {
		return nil
	}
	return fmt.Errorf("accept %q does not include %q", accept, mime)
}

func RequireContentType(r *http.Request, mime string) error {
	if mime == "" || mime == "application/json" {
		return nil
	}
	contentType := r.Header.Get("Content-Type")
	if contentType == "" {
		return fmt.Errorf("missing content type %q", mime)
	}
	mediaType := strings.TrimSpace(strings.Split(contentType, ";")[0])
	if !strings.EqualFold(mediaType, mime) {
		return fmt.Errorf("content type %q does not match %q", mediaType, mime)
	}
	return nil
}

func ParseString(value string) (string, error) {
	if value == "" {
		return "", errors.New("missing string value")
	}
	return value, nil
}

func ParseOptionalString(value string) *string {
	if value == "" {
		return nil
	}
	copy := value
	return &copy
}

func ParseBool(value string) (bool, error) {
	return strconv.ParseBool(value)
}

func ParseUint32(value string) (uint32, error) {
	parsed, err := strconv.ParseUint(value, 10, 32)
	return uint32(parsed), err
}

func ParseInt32(value string) (int32, error) {
	parsed, err := strconv.ParseInt(value, 10, 32)
	return int32(parsed), err
}

func ParseUint64(value string) (uint64, error) {
	return strconv.ParseUint(value, 10, 64)
}

func ParseInt64(value string) (int64, error) {
	return strconv.ParseInt(value, 10, 64)
}

func PathString(r *http.Request, key string) (string, error) {
	return ParseString(r.PathValue(key))
}

func QueryString(values url.Values, key string) (string, error) {
	return ParseString(values.Get(key))
}

func QueryOptionalString(values url.Values, key string) *string {
	return ParseOptionalString(values.Get(key))
}

func HeaderString(h http.Header, key string) (string, error) {
	return ParseString(h.Get(key))
}

func HeaderStrings(h http.Header, key string) []string {
	return h.Values(key)
}

func CookieString(r *http.Request, key string) (string, error) {
	cookie, err := r.Cookie(key)
	if err != nil {
		return "", err
	}
	return cookie.Value, nil
}

func FormatString(value string) string {
	return value
}

func FormatBool(value bool) string {
	return strconv.FormatBool(value)
}

func FormatUint32(value uint32) string {
	return strconv.FormatUint(uint64(value), 10)
}

func FormatInt32(value int32) string {
	return strconv.FormatInt(int64(value), 10)
}

func FormatUint64(value uint64) string {
	return strconv.FormatUint(value, 10)
}

func FormatInt64(value int64) string {
	return strconv.FormatInt(value, 10)
}
