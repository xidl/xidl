package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyCorsAny struct{}
func (s *MyCorsAny) Hello(ctx context.Context, req *CorsAnyHelloRequest) (*CorsAnyHelloResponse, error) {
	return &CorsAnyHelloResponse{Return: "any"}, nil
}

type MyCorsSpecific struct{}
func (s *MyCorsSpecific) Trusted(ctx context.Context, req *CorsSpecificTrustedRequest) (*CorsSpecificTrustedResponse, error) {
	return &CorsSpecificTrustedResponse{Return: "trusted"}, nil
}

func main() {
	r := gin.Default()
	RegisterCorsAnyHandler(r, &MyCorsAny{})
	RegisterCorsSpecificHandler(r, &MyCorsSpecific{})
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
