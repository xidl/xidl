package main

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyRepro struct{}

func (s *MyRepro) FlattenAny(ctx context.Context, req *Issue171ReproServiceFlattenAnyRequest) (*Issue171ReproServiceFlattenAnyResponse, error) {
	m, ok := req.Payload.(map[string]any)
	if !ok || m["foo"] != "bar" {
		return nil, errors.New("invalid payload")
	}
	return &Issue171ReproServiceFlattenAnyResponse{}, nil
}
func (s *MyRepro) FlattenStructWithAny(ctx context.Context, req *Issue171ReproServiceFlattenStructWithAnyRequest) (*Issue171ReproServiceFlattenStructWithAnyResponse, error) {
	m, ok := req.Payload.Field.(map[string]any)
	if !ok || m["foo"] != "bar" {
		return nil, errors.New("invalid payload")
	}
	return &Issue171ReproServiceFlattenStructWithAnyResponse{}, nil
}
func main() {
	r := gin.Default()
	svc := &MyRepro{}
	RegisterIssue171ReproServiceHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
