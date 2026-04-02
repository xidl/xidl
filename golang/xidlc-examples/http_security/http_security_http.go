package http_security

import (
	"context"
	"fmt"
	"net/http"
	"strings"

	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-http"
)

type HttpSecurityApiService interface {
	GetReport(ctx context.Context, req *HttpSecurityApiGetReportRequest) (*HttpSecurityApiGetReportResponse, error)
	UpdateReport(ctx context.Context, req *HttpSecurityApiUpdateReportRequest) (*HttpSecurityApiUpdateReportResponse, error)
	Health(ctx context.Context, req *HttpSecurityApiHealthRequest) (*HttpSecurityApiHealthResponse, error)
}

func NewHttpSecurityApiHandler(svc HttpSecurityApiService) http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("GET /reports/{id}", func(w http.ResponseWriter, r *http.Request) {
		if err := xidlgohttp.RequireAccept(r, "application/json"); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusNotAcceptable, "NOT_ACCEPTABLE", err.Error())
			return
		}
		ctx, err := xidlgohttp.RequireAuth(r, HttpSecurityApiGetReportSecurityRequirements())
		if err != nil {
			xidlgohttp.Unauthorized(w, HttpSecurityApiGetReportSecurityRequirements())
			return
		}
		r = r.WithContext(ctx)
		req := &HttpSecurityApiGetReportRequest{}
		if value, err := xidlgohttp.PathString(r, "id"); err == nil {
			req.Id = value
		} else {
			xidlgohttp.WriteJSONError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
			return
		}
		if value, err := xidlgohttp.HeaderString(r.Header, "X-Trace-Id"); err == nil {
			req.TraceId = value
		} else {
			xidlgohttp.WriteJSONError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
			return
		}
		resp, err := svc.GetReport(r.Context(), req)
		if err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
			return
		}
		if err := xidlgohttp.EncodeBody(w, "application/json", resp.Return); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "ENCODE", err.Error())
		}
	})

	mux.HandleFunc("POST /reports/{id}", func(w http.ResponseWriter, r *http.Request) {
		if err := xidlgohttp.RequireAccept(r, "application/json"); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusNotAcceptable, "NOT_ACCEPTABLE", err.Error())
			return
		}
		ctx, err := xidlgohttp.RequireAuth(r, HttpSecurityApiUpdateReportSecurityRequirements())
		if err != nil {
			xidlgohttp.Unauthorized(w, HttpSecurityApiUpdateReportSecurityRequirements())
			return
		}
		r = r.WithContext(ctx)
		req := &HttpSecurityApiUpdateReportRequest{}
		if value, err := xidlgohttp.PathString(r, "id"); err == nil {
			req.Id = value
		} else {
			xidlgohttp.WriteJSONError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
			return
		}
		var body string
		if err := xidlgohttp.DecodeBody(r, "application/json", &body); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusBadRequest, "BAD_REQUEST", err.Error())
			return
		}
		req.Body = body
		resp, err := svc.UpdateReport(r.Context(), req)
		if err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
			return
		}
		if err := xidlgohttp.EncodeBody(w, "application/json", resp.Return); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "ENCODE", err.Error())
		}
	})

	mux.HandleFunc("GET /health", func(w http.ResponseWriter, r *http.Request) {
		if err := xidlgohttp.RequireAccept(r, "application/json"); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusNotAcceptable, "NOT_ACCEPTABLE", err.Error())
			return
		}
		resp, err := svc.Health(r.Context(), &HttpSecurityApiHealthRequest{})
		if err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
			return
		}
		if err := xidlgohttp.EncodeBody(w, "application/json", resp.Return); err != nil {
			xidlgohttp.WriteJSONError(w, http.StatusInternalServerError, "ENCODE", err.Error())
		}
	})

	return mux
}

type HttpSecurityApiClient struct {
	baseURL    string
	httpClient *http.Client
	auth       xidlgohttp.ClientAuth
}

func NewHttpSecurityApiClient(baseURL string, httpClient *http.Client, auth xidlgohttp.ClientAuth) *HttpSecurityApiClient {
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	return &HttpSecurityApiClient{baseURL: strings.TrimRight(baseURL, "/"), httpClient: httpClient, auth: auth}
}

func (c *HttpSecurityApiClient) GetReport(ctx context.Context, req *HttpSecurityApiGetReportRequest) (*HttpSecurityApiGetReportResponse, error) {
	requestURL := c.baseURL + formatHttpSecurityApiGetReportPath(req)
	httpReq, err := http.NewRequestWithContext(ctx, "GET", requestURL, nil)
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Accept", "application/json")
	httpReq.Header.Set("X-Trace-Id", req.TraceId)
	xidlgohttp.ApplyClientAuth(httpReq, c.auth, HttpSecurityApiGetReportSecurityRequirements())
	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("http %d", resp.StatusCode)
	}
	decoded, err := decodeHttpSecurityApiGetReportResponse(resp)
	if err != nil {
		return nil, err
	}
	return &decoded, nil
}

func (c *HttpSecurityApiClient) UpdateReport(ctx context.Context, req *HttpSecurityApiUpdateReportRequest) (*HttpSecurityApiUpdateReportResponse, error) {
	requestURL := c.baseURL + formatHttpSecurityApiUpdateReportPath(req)
	var requestBody strings.Builder
	if err := xidlgohttp.MustCodecForMime("application/json").Encode(&requestBody, req.Body); err != nil {
		return nil, err
	}
	httpReq, err := http.NewRequestWithContext(ctx, "POST", requestURL, strings.NewReader(requestBody.String()))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Accept", "application/json")
	xidlgohttp.ApplyClientAuth(httpReq, c.auth, HttpSecurityApiUpdateReportSecurityRequirements())
	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("http %d", resp.StatusCode)
	}
	decoded, err := decodeHttpSecurityApiUpdateReportResponse(resp)
	if err != nil {
		return nil, err
	}
	return &decoded, nil
}

func (c *HttpSecurityApiClient) Health(ctx context.Context, req *HttpSecurityApiHealthRequest) (*HttpSecurityApiHealthResponse, error) {
	requestURL := c.baseURL + "/health"
	httpReq, err := http.NewRequestWithContext(ctx, "GET", requestURL, nil)
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Accept", "application/json")
	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("http %d", resp.StatusCode)
	}
	decoded, err := decodeHttpSecurityApiHealthResponse(resp)
	if err != nil {
		return nil, err
	}
	return &decoded, nil
}

type HttpSecurityApiGetReportRequest struct {
	Id      string `json:"id"`
	TraceId string `json:"trace_id"`
}

type HttpSecurityApiGetReportResponse struct {
	Return string `json:"return"`
}

type HttpSecurityApiUpdateReportRequest struct {
	Id   string `json:"id"`
	Body string `json:"body"`
}

type HttpSecurityApiUpdateReportResponse struct {
	Return string `json:"return"`
}

type HttpSecurityApiHealthRequest struct{}

type HttpSecurityApiHealthResponse struct {
	Return string `json:"return"`
}

func HttpSecurityApiGetReportSecurityRequirements() []xidlgohttp.SecurityRequirement {
	return []xidlgohttp.SecurityRequirement{
		{Kind: xidlgohttp.SecurityBasic, Realm: "GetReport"},
		{Kind: xidlgohttp.SecurityAPIKey, Location: xidlgohttp.ApiKeyHeader, Name: "X-Org-Key"},
	}
}

func HttpSecurityApiUpdateReportSecurityRequirements() []xidlgohttp.SecurityRequirement {
	return []xidlgohttp.SecurityRequirement{
		{Kind: xidlgohttp.SecurityOAuth2, Scopes: []string{"reports:write"}},
	}
}

func HttpSecurityApiHealthSecurityRequirements() []xidlgohttp.SecurityRequirement {
	return []xidlgohttp.SecurityRequirement{}
}

func HttpSecurityApiGetReportDeprecated() xidlgohttp.DeprecatedInfo {
	return xidlgohttp.DeprecatedInfo{}
}

func HttpSecurityApiUpdateReportDeprecated() xidlgohttp.DeprecatedInfo {
	return xidlgohttp.DeprecatedInfo{}
}

func HttpSecurityApiHealthDeprecated() xidlgohttp.DeprecatedInfo {
	return xidlgohttp.DeprecatedInfo{}
}

func formatHttpSecurityApiGetReportPath(req *HttpSecurityApiGetReportRequest) string {
	return strings.ReplaceAll("/reports/{id}", "{id}", req.Id)
}

func formatHttpSecurityApiUpdateReportPath(req *HttpSecurityApiUpdateReportRequest) string {
	return strings.ReplaceAll("/reports/{id}", "{id}", req.Id)
}

func decodeHttpSecurityApiGetReportResponse(resp *http.Response) (HttpSecurityApiGetReportResponse, error) {
	out := HttpSecurityApiGetReportResponse{}
	var body string
	if err := xidlgohttp.MustCodecForMime("application/json").Decode(resp.Body, &body); err != nil {
		return out, err
	}
	out.Return = body
	return out, nil
}

func decodeHttpSecurityApiUpdateReportResponse(resp *http.Response) (HttpSecurityApiUpdateReportResponse, error) {
	out := HttpSecurityApiUpdateReportResponse{}
	var body string
	if err := xidlgohttp.MustCodecForMime("application/json").Decode(resp.Body, &body); err != nil {
		return out, err
	}
	out.Return = body
	return out, nil
}

func decodeHttpSecurityApiHealthResponse(resp *http.Response) (HttpSecurityApiHealthResponse, error) {
	out := HttpSecurityApiHealthResponse{}
	var body string
	if err := xidlgohttp.MustCodecForMime("application/json").Decode(resp.Body, &body); err != nil {
		return out, err
	}
	out.Return = body
	return out, nil
}
