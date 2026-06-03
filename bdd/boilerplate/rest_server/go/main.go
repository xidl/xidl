package main

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"
	"strings"
	"sync"

	"github.com/gin-gonic/gin"
)

type MyRestServer struct {
	host       string
	serverName string
	userInfo   sync.Map
	keyStore   sync.Map
}


func (s *MyRestServer) GetServerName(ctx context.Context, req *RestServerGetServerNameRequest) (*RestServerGetServerNameResponse, error) {
	return &RestServerGetServerNameResponse{Return: s.serverName}, nil
}

func (s *MyRestServer) SetServerName(ctx context.Context, req *RestServerSetServerNameRequest) (*RestServerSetServerNameResponse, error) {
	s.serverName = req.Name
	return &RestServerSetServerNameResponse{}, nil
}

func (s *MyRestServer) GetUserInfo(ctx context.Context, req *RestServerGetUserInfoRequest) (*RestServerGetUserInfoResponse, error) {
	val, ok := s.userInfo.Load(req.Id)
	if !ok {
		return nil, errors.New("user not found")
	}
	return &RestServerGetUserInfoResponse{Return: *val.(*UserInfo)}, nil
}

func (s *MyRestServer) QueryUserInfo(ctx context.Context, req *RestServerQueryUserInfoRequest) (*RestServerQueryUserInfoResponse, error) {
	val, ok := s.userInfo.Load(req.Id)
	if !ok {
		return nil, errors.New("user not found")
	}
	return &RestServerQueryUserInfoResponse{Return: *val.(*UserInfo)}, nil
}

func (s *MyRestServer) PostUserInfo(ctx context.Context, req *RestServerPostUserInfoRequest) (*RestServerPostUserInfoResponse, error) {
	s.userInfo.Store(req.Id, &req.Info)
	return &RestServerPostUserInfoResponse{}, nil
}

func (s *MyRestServer) PutKeyValue(ctx context.Context, req *RestServerPutKeyValueRequest) (*RestServerPutKeyValueResponse, error) {
	s.keyStore.Store(req.Key, req.Value)
	return &RestServerPutKeyValueResponse{}, nil
}

func (s *MyRestServer) DeleteKey(ctx context.Context, req *RestServerDeleteKeyRequest) (*RestServerDeleteKeyResponse, error) {
	s.keyStore.Delete(req.Key)
	return &RestServerDeleteKeyResponse{}, nil
}

func (s *MyRestServer) PatchKey(ctx context.Context, req *RestServerPatchKeyRequest) (*RestServerPatchKeyResponse, error) {
	s.keyStore.Store(req.Key, req.Value)
	return &RestServerPatchKeyResponse{}, nil
}

func (s *MyRestServer) IsKeyExists(ctx context.Context, req *RestServerIsKeyExistsRequest) (*RestServerIsKeyExistsResponse, error) {
	_, ok := s.keyStore.Load(req.KeyAlias)
	if !ok {
		return nil, errors.New("key not found")
	}
	return &RestServerIsKeyExistsResponse{}, nil
}

func (s *MyRestServer) GetKeyOptions(ctx context.Context, req *RestServerGetKeyOptionsRequest) (*RestServerGetKeyOptionsResponse, error) {
	_, ok := s.keyStore.Load(req.Key)
	return &RestServerGetKeyOptionsResponse{Exists: ok}, nil
}

func (s *MyRestServer) GetKey1(ctx context.Context, req *RestServerGetKey1Request) (*RestServerGetKey1Response, error) {
	val, ok := s.keyStore.Load(req.Key)
	if !ok {
		return nil, errors.New("key not found")
	}
	return &RestServerGetKey1Response{Value: val.(string)}, nil
}

func (s *MyRestServer) GetKey2(ctx context.Context, req *RestServerGetKey2Request) (*RestServerGetKey2Response, error) {
	val, ok := s.keyStore.Load(req.Key)
	if !ok {
		return nil, errors.New("key not found")
	}
	return &RestServerGetKey2Response{Value: val.(string)}, nil
}

func (s *MyRestServer) GetKey3(ctx context.Context, req *RestServerGetKey3Request) (*RestServerGetKey3Response, error) {
	val, ok := s.keyStore.Load(req.Key)
	if !ok {
		return nil, errors.New("key not found")
	}
	return &RestServerGetKey3Response{Value: val.(string)}, nil
}

func (s *MyRestServer) GetKey4(ctx context.Context, req *RestServerGetKey4Request) (*RestServerGetKey4Response, error) {
	val, ok := s.keyStore.Load(req.Key)
	if !ok {
		return nil, errors.New("key not found")
	}
	return &RestServerGetKey4Response{Value: val.(string)}, nil
}

func (s *MyRestServer) Login(ctx context.Context, req *RestServerLoginRequest) (*RestServerLoginResponse, error) {
	return &RestServerLoginResponse{SessionId: "simple_session_id"}, nil
}

func (s *MyRestServer) LoginRealm(ctx context.Context, req *RestServerLoginRealmRequest) (*RestServerLoginRealmResponse, error) {
	return &RestServerLoginRealmResponse{SessionId: "simple_session_id"}, nil
}

func (s *MyRestServer) IsLogined(ctx context.Context, req *RestServerIsLoginedRequest) (*RestServerIsLoginedResponse, error) {
	return &RestServerIsLoginedResponse{Return: req.SessionId != ""}, nil
}

func (s *MyRestServer) LoginBearer(ctx context.Context, req *RestServerLoginBearerRequest) (*RestServerLoginBearerResponse, error) {
	return &RestServerLoginBearerResponse{}, nil
}

func (s *MyRestServer) GetTimestamp(ctx context.Context, req *RestServerGetTimestampRequest) (*RestServerGetTimestampResponse, error) {
	return &RestServerGetTimestampResponse{}, nil
}

func (s *MyRestServer) IsAdmin(ctx context.Context, req *RestServerIsAdminRequest) (*RestServerIsAdminResponse, error) {
	return &RestServerIsAdminResponse{}, nil
}

type bodyLogWriter struct {
	gin.ResponseWriter
	body   *bytes.Buffer
	status int
}

func (w *bodyLogWriter) Write(b []byte) (int, error) {
	return w.body.Write(b)
}

func (w *bodyLogWriter) WriteString(s string) (int, error) {
	return w.body.WriteString(s)
}

func (w *bodyLogWriter) WriteHeader(statusCode int) {
	w.status = statusCode
}

func (w *bodyLogWriter) Status() int {
	if w.status == 0 {
		return w.ResponseWriter.Status()
	}
	return w.status
}

func ErrorMappingMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		blw := &bodyLogWriter{body: bytes.NewBuffer(nil), ResponseWriter: c.Writer}
		c.Writer = blw
		c.Next()

		status := blw.Status()
		if status == http.StatusInternalServerError && (strings.Contains(blw.body.String(), "not found") || strings.Contains(blw.body.String(), "key not found")) {
			blw.ResponseWriter.WriteHeader(http.StatusNotFound)
			_, _ = blw.ResponseWriter.Write([]byte(`{"code":404,"msg":"Not Found"}`))
		} else {
			if status != 0 {
				blw.ResponseWriter.WriteHeader(status)
			}
			_, _ = blw.ResponseWriter.Write(blw.body.Bytes())
		}
	}
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.HandleMethodNotAllowed = true
	r.Use(gin.Recovery(), ErrorMappingMiddleware())
	svc := &MyRestServer{
		host:       "localhost",
		serverName: "rest_server",
	}
	RegisterRestServerHandler(r, svc)
	r.GET("/attribute/host", func(c *gin.Context) {
		c.JSON(200, svc.host)
	})
	r.POST("/attribute/host", func(c *gin.Context) {
		var body struct {
			Host string `json:"host"`
		}
		if err := c.ShouldBindJSON(&body); err != nil {
			c.Status(400)
			return
		}
		svc.host = body.Host
		c.Status(204)
	})
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
