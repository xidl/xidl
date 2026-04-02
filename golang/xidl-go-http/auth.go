package xidlgohttp

import (
	"context"
	"encoding/base64"
	"errors"
	"net/http"
	"net/url"
	"strings"
)

type BasicAuth struct {
	Username string
	Password string
}

type ApiKeyLocation string

const (
	ApiKeyHeader ApiKeyLocation = "header"
	ApiKeyQuery  ApiKeyLocation = "query"
	ApiKeyCookie ApiKeyLocation = "cookie"
)

type ApiKeyAuth struct {
	Location ApiKeyLocation
	Name     string
	Value    string
}

type ClientAuth struct {
	Basic   *BasicAuth
	Bearer  string
	APIKeys []ApiKeyAuth
}

type SecurityKind string

const (
	SecurityBasic  SecurityKind = "basic"
	SecurityBearer SecurityKind = "bearer"
	SecurityAPIKey SecurityKind = "api_key"
	SecurityOAuth2 SecurityKind = "oauth2"
	SecurityNone   SecurityKind = "none"
)

type SecurityRequirement struct {
	Kind     SecurityKind
	Name     string
	Location ApiKeyLocation
	Realm    string
	Scopes   []string
}

type contextKey string

const (
	basicAuthContextKey contextKey = "xidl.basic_auth"
	bearerContextKey    contextKey = "xidl.bearer"
	apiKeysContextKey   contextKey = "xidl.api_keys"
)

func ApplyClientAuth(req *http.Request, auth ClientAuth, requirements []SecurityRequirement) {
	for _, requirement := range requirements {
		switch requirement.Kind {
		case SecurityBasic:
			if auth.Basic != nil {
				token := base64.StdEncoding.EncodeToString([]byte(auth.Basic.Username + ":" + auth.Basic.Password))
				req.Header.Set("Authorization", "Basic "+token)
			}
		case SecurityBearer:
			if auth.Bearer != "" {
				req.Header.Set("Authorization", "Bearer "+auth.Bearer)
			}
		case SecurityOAuth2:
			if auth.Bearer != "" {
				req.Header.Set("Authorization", "Bearer "+auth.Bearer)
			}
		case SecurityAPIKey:
			for _, key := range auth.APIKeys {
				if key.Location != requirement.Location || key.Name != requirement.Name {
					continue
				}
				switch key.Location {
				case ApiKeyHeader:
					req.Header.Set(key.Name, key.Value)
				case ApiKeyQuery:
					query := req.URL.Query()
					query.Set(key.Name, key.Value)
					req.URL.RawQuery = query.Encode()
				case ApiKeyCookie:
					req.AddCookie(&http.Cookie{Name: key.Name, Value: key.Value})
				}
			}
		}
	}
}

func RequireAuth(r *http.Request, requirements []SecurityRequirement) (context.Context, error) {
	if len(requirements) == 0 {
		return r.Context(), nil
	}

	ctx := r.Context()
	authz := r.Header.Get("Authorization")
	var basic *BasicAuth
	var bearer string
	if strings.HasPrefix(authz, "Basic ") {
		raw, err := base64.StdEncoding.DecodeString(strings.TrimPrefix(authz, "Basic "))
		if err == nil {
			username, password, ok := strings.Cut(string(raw), ":")
			if ok {
				basic = &BasicAuth{Username: username, Password: password}
			}
		}
	}
	if strings.HasPrefix(authz, "Bearer ") {
		bearer = strings.TrimPrefix(authz, "Bearer ")
	}

	foundAPIKeys := make([]ApiKeyAuth, 0)
	for _, requirement := range requirements {
		switch requirement.Kind {
		case SecurityBasic:
			if basic != nil {
				return context.WithValue(ctx, basicAuthContextKey, *basic), nil
			}
		case SecurityBearer:
			if bearer != "" {
				return context.WithValue(ctx, bearerContextKey, bearer), nil
			}
		case SecurityOAuth2:
			if bearer != "" {
				return context.WithValue(ctx, bearerContextKey, bearer), nil
			}
		case SecurityAPIKey:
			if value, ok := findAPIKey(r, requirement); ok {
				foundAPIKeys = append(foundAPIKeys, ApiKeyAuth{
					Location: requirement.Location,
					Name:     requirement.Name,
					Value:    value,
				})
			}
		}
	}
	if len(foundAPIKeys) > 0 {
		return context.WithValue(ctx, apiKeysContextKey, foundAPIKeys), nil
	}
	return ctx, errors.New("missing required authentication")
}

func Unauthorized(w http.ResponseWriter, requirements []SecurityRequirement) {
	for _, requirement := range requirements {
		if requirement.Kind == SecurityBasic {
			realm := requirement.Realm
			if realm == "" {
				realm = "xidl"
			}
			w.Header().Set("WWW-Authenticate", `Basic realm="`+realm+`"`)
			break
		}
		if requirement.Kind == SecurityBearer {
			w.Header().Set("WWW-Authenticate", "Bearer")
			break
		}
		if requirement.Kind == SecurityOAuth2 {
			w.Header().Set("WWW-Authenticate", "Bearer")
			break
		}
	}
	http.Error(w, http.StatusText(http.StatusUnauthorized), http.StatusUnauthorized)
}

func BasicAuthFromContext(ctx context.Context) (BasicAuth, bool) {
	value, ok := ctx.Value(basicAuthContextKey).(BasicAuth)
	return value, ok
}

func BearerFromContext(ctx context.Context) (string, bool) {
	value, ok := ctx.Value(bearerContextKey).(string)
	return value, ok
}

func APIKeysFromContext(ctx context.Context) ([]ApiKeyAuth, bool) {
	value, ok := ctx.Value(apiKeysContextKey).([]ApiKeyAuth)
	return value, ok
}

func AddQueryValue(rawURL, key, value string) string {
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return rawURL
	}
	query := parsed.Query()
	query.Set(key, value)
	parsed.RawQuery = query.Encode()
	return parsed.String()
}

func findAPIKey(r *http.Request, requirement SecurityRequirement) (string, bool) {
	switch requirement.Location {
	case ApiKeyHeader:
		value := r.Header.Get(requirement.Name)
		return value, value != ""
	case ApiKeyQuery:
		value := r.URL.Query().Get(requirement.Name)
		return value, value != ""
	case ApiKeyCookie:
		cookie, err := r.Cookie(requirement.Name)
		if err != nil {
			return "", false
		}
		return cookie.Value, cookie.Value != ""
	default:
		return "", false
	}
}
