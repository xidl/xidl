package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyStore struct{}

func (s *MyStore) Echo(ctx context.Context, req *StoreEchoRequest) (*StoreEchoResponse, error) {
	return &StoreEchoResponse{Return: req.Item}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Recovery())
	svc := &MyStore{}
	RegisterStoreHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
