package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MySmartCityRestService struct{}

func (s *MySmartCityRestService) GetStopEta(ctx context.Context, req *SmartCityRestApiGetStopEtaRequest) (*SmartCityRestApiGetStopEtaResponse, error) {
	return &SmartCityRestApiGetStopEtaResponse{
		Return:      req.StopId,
		EtaSeconds:  240,
		Destination: "Central Station",
	}, nil
}

func (s *MySmartCityRestService) ListNearbyStops(ctx context.Context, req *SmartCityRestApiListNearbyStopsRequest) (*SmartCityRestApiListNearbyStopsResponse, error) {
	return &SmartCityRestApiListNearbyStopsResponse{
		Return: []string{req.StopId + "-A", req.StopId + "-B"},
	}, nil
}

func (s *MySmartCityRestService) DownloadAsset(ctx context.Context, req *SmartCityRestApiDownloadAssetRequest) (*SmartCityRestApiDownloadAssetResponse, error) {
	return &SmartCityRestApiDownloadAssetResponse{
		Return:      []byte("asset:" + req.Path),
		ContentType: "text/plain",
		Etag:        "etag-demo",
	}, nil
}

func (s *MySmartCityRestService) ProbeLot(ctx context.Context, req *SmartCityRestApiProbeLotRequest) (*SmartCityRestApiProbeLotResponse, error) {
	return &SmartCityRestApiProbeLotResponse{}, nil
}

func (s *MySmartCityRestService) ReserveLot(ctx context.Context, req *SmartCityRestApiReserveLotRequest) (*SmartCityRestApiReserveLotResponse, error) {
	return &SmartCityRestApiReserveLotResponse{
		Return:           "resv-" + req.LotId,
		ReservationState: "CONFIRMED",
		ExpiresAt:        "2026-03-08T10:00:00Z",
	}, nil
}

func (s *MySmartCityRestService) CancelReservation(ctx context.Context, req *SmartCityRestApiCancelReservationRequest) (*SmartCityRestApiCancelReservationResponse, error) {
	return &SmartCityRestApiCancelReservationResponse{}, nil
}

func (s *MySmartCityRestService) GetProfile(ctx context.Context, req *SmartCityRestApiGetProfileRequest) (*SmartCityRestApiGetProfileResponse, error) {
	return &SmartCityRestApiGetProfileResponse{
		Return:      req.CitizenId,
		DisplayName: "Taylor",
		PhoneNumber: "+1-555-0101",
		Language:    "en-US",
	}, nil
}

func (s *MySmartCityRestService) UpdateProfile(ctx context.Context, req *SmartCityRestApiUpdateProfileRequest) (*SmartCityRestApiUpdateProfileResponse, error) {
	return &SmartCityRestApiUpdateProfileResponse{
		AuditId: "audit-20260307-001",
	}, nil
}

func (s *MySmartCityRestService) GetDeviceStatus(ctx context.Context, req *SmartCityRestApiGetDeviceStatusRequest) (*SmartCityRestApiGetDeviceStatusResponse, error) {
	return &SmartCityRestApiGetDeviceStatusResponse{
		Return:      "device:" + req.DeviceId,
		TraceEcho:   req.XTraceId,
		SessionEcho: req.Session,
	}, nil
}

func (s *MySmartCityRestService) GetAttributeApiVersion(ctx context.Context, req *SmartCityRestApiGetAttributeApiVersionRequest) (*SmartCityRestApiGetAttributeApiVersionResponse, error) {
	return &SmartCityRestApiGetAttributeApiVersionResponse{
		Return: "v2.0.0",
	}, nil
}

func (s *MySmartCityRestService) GetAttributeMaintenanceMode(ctx context.Context, req *SmartCityRestApiGetAttributeMaintenanceModeRequest) (*SmartCityRestApiGetAttributeMaintenanceModeResponse, error) {
	return &SmartCityRestApiGetAttributeMaintenanceModeResponse{
		Return: false,
	}, nil
}

func (s *MySmartCityRestService) SetAttributeMaintenanceMode(ctx context.Context, req *SmartCityRestApiSetAttributeMaintenanceModeRequest) (*SmartCityRestApiSetAttributeMaintenanceModeResponse, error) {
	return &SmartCityRestApiSetAttributeMaintenanceModeResponse{}, nil
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()
	svc := &MySmartCityRestService{}
	RegisterSmartCityRestApiHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
