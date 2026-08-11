package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyReservedWordService struct{}

func (s *MyReservedWordService) GetMonitor(
	ctx context.Context,
	req *ReservedWordServiceGetMonitorRequest,
) (*ReservedWordServiceGetMonitorResponse, error) {
	return &ReservedWordServiceGetMonitorResponse{
		Return: fmt.Sprintf("monitor:%s:%s", req.Id, req.Type),
	}, nil
}

func (s *MyReservedWordService) Search(
	ctx context.Context,
	req *ReservedWordServiceSearchRequest,
) (*ReservedWordServiceSearchResponse, error) {
	return &ReservedWordServiceSearchResponse{
		Return: fmt.Sprintf("search:%s", req.Type),
	}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Recovery())
	svc := &MyReservedWordService{}
	RegisterReservedWordServiceHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
