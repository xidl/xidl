package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"sync"

	"github.com/gin-gonic/gin"
)

type MyAllScenarios struct {
	status Status
	sync.Mutex
}

func (s *MyAllScenarios) GetItem(ctx context.Context, req *AllScenariosServiceGetItemRequest) (*AllScenariosServiceGetItemResponse, error) {
	return &AllScenariosServiceGetItemResponse{Return: fmt.Sprintf("Item %d with %s and %s", req.Id, req.Filter, req.TraceId)}, nil
}
func (s *MyAllScenarios) CreateItem(ctx context.Context, req *AllScenariosServiceCreateItemRequest) (*AllScenariosServiceCreateItemResponse, error) {
	return &AllScenariosServiceCreateItemResponse{Return: 42}, nil
}
func (s *MyAllScenarios) UpdateItem(ctx context.Context, req *AllScenariosServiceUpdateItemRequest) (*AllScenariosServiceUpdateItemResponse, error) {
	return &AllScenariosServiceUpdateItemResponse{}, nil
}
func (s *MyAllScenarios) DeleteItem(ctx context.Context, req *AllScenariosServiceDeleteItemRequest) (*AllScenariosServiceDeleteItemResponse, error) {
	return &AllScenariosServiceDeleteItemResponse{}, nil
}
func (s *MyAllScenarios) GetAttributeSystemStatus(ctx context.Context, req *AllScenariosServiceGetAttributeSystemStatusRequest) (*AllScenariosServiceGetAttributeSystemStatusResponse, error) {
	s.Lock()
	defer s.Unlock()
	return &AllScenariosServiceGetAttributeSystemStatusResponse{Return: s.status}, nil
}
func (s *MyAllScenarios) SetAttributeSystemStatus(ctx context.Context, req *AllScenariosServiceSetAttributeSystemStatusRequest) (*AllScenariosServiceSetAttributeSystemStatusResponse, error) {
	s.Lock()
	defer s.Unlock()
	s.status = req.SystemStatus
	return &AllScenariosServiceSetAttributeSystemStatusResponse{}, nil
}
func (s *MyAllScenarios) GetAttributeVersion(ctx context.Context, req *AllScenariosServiceGetAttributeVersionRequest) (*AllScenariosServiceGetAttributeVersionResponse, error) {
	return &AllScenariosServiceGetAttributeVersionResponse{Return: "1.0.0"}, nil
}
func (s *MyAllScenarios) UploadForm(ctx context.Context, req *AllScenariosServiceUploadFormRequest) (*AllScenariosServiceUploadFormResponse, error) {
	return &AllScenariosServiceUploadFormResponse{}, nil
}
func (s *MyAllScenarios) SecureData(ctx context.Context, req *AllScenariosServiceSecureDataRequest) (*AllScenariosServiceSecureDataResponse, error) {
	return &AllScenariosServiceSecureDataResponse{Return: "Secret"}, nil
}
func main() {
	r := gin.Default()
	svc := &MyAllScenarios{status: StatusActive}
	RegisterAllScenariosServiceHandler(r, svc)
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
