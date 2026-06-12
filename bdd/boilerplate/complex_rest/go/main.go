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

type MyUserService struct {
	users sync.Map
}

func (s *MyUserService) GetUser(ctx context.Context, req *UserServiceGetUserRequest) (*UserServiceGetUserResponse, error) {
	val, ok := s.users.Load(req.Id)
	if !ok {
		return nil, errors.New("user not found")
	}
	return &UserServiceGetUserResponse{Return: *val.(*User)}, nil
}

func (s *MyUserService) CreateUser(ctx context.Context, req *UserServiceCreateUserRequest) (*UserServiceCreateUserResponse, error) {
	s.users.Store(req.User.Id, &req.User)
	return &UserServiceCreateUserResponse{Return: req.User}, nil
}

func (s *MyUserService) ListUsers(ctx context.Context, req *UserServiceListUsersRequest) (*UserServiceListUsersResponse, error) {
	var users []User
	s.users.Range(func(k, v interface{}) bool {
		users = append(users, *v.(*User))
		return true
	})
	return &UserServiceListUsersResponse{Return: users}, nil
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
		if status == http.StatusInternalServerError && strings.Contains(blw.body.String(), "user not found") {
			blw.ResponseWriter.WriteHeader(http.StatusNotFound)
			_, _ = blw.ResponseWriter.Write([]byte(`{"code":404,"msg":"user not found"}`))
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
	r.Use(gin.Recovery(), ErrorMappingMiddleware())
	svc := &MyUserService{}
	RegisterUserServiceHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
