package main

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"

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
		Return:      []byte("asset:" + req.AssetPath),
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

func (s *MySmartCityRestService) GetAttributeApiVersion(ctx context.Context, req *SmartCityRestApiGetAttributeApiVersionRequest) (*SmartCityRestApiGetAttributeApiVersionResponse, error) {
	return &SmartCityRestApiGetAttributeApiVersionResponse{Return: "v2.0.0"}, nil
}

func (s *MySmartCityRestService) GetAttributeMaintenanceMode(ctx context.Context, req *SmartCityRestApiGetAttributeMaintenanceModeRequest) (*SmartCityRestApiGetAttributeMaintenanceModeResponse, error) {
	return &SmartCityRestApiGetAttributeMaintenanceModeResponse{Return: false}, nil
}

func (s *MySmartCityRestService) SetAttributeMaintenanceMode(ctx context.Context, req *SmartCityRestApiSetAttributeMaintenanceModeRequest) (*SmartCityRestApiSetAttributeMaintenanceModeResponse, error) {
	return &SmartCityRestApiSetAttributeMaintenanceModeResponse{}, nil
}

func (s *MySmartCityRestService) GetDeviceStatus(ctx context.Context, req *SmartCityRestApiGetDeviceStatusRequest) (*SmartCityRestApiGetDeviceStatusResponse, error) {
	return &SmartCityRestApiGetDeviceStatusResponse{
		Return:      "device:" + req.DeviceId,
		TraceEcho:   req.TraceId,
		SessionEcho: req.SessionId,
	}, nil
}

type bodyWriter struct {
	gin.ResponseWriter
	body *bytes.Buffer
}

func (w *bodyWriter) Write(b []byte) (int, error) {
	return w.body.Write(b)
}

func MsgpackAndBytesMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		if strings.HasPrefix(c.Request.URL.Path, "/v1/assets/") {
			w := &bodyWriter{body: &bytes.Buffer{}, ResponseWriter: c.Writer}
			c.Writer = w
			c.Next()

			if strings.Contains(w.Header().Get("Content-Type"), "application/json") {
				var data map[string]interface{}
				if err := json.Unmarshal(w.body.Bytes(), &data); err == nil {
					if retStr, ok := data["return"].(string); ok {
						if decoded, err := base64.StdEncoding.DecodeString(retStr); err == nil {
							ints := make([]int, len(decoded))
							for i, b := range decoded {
								ints[i] = int(b)
							}
							data["return"] = ints
							newBody, _ := json.Marshal(data)
							w.ResponseWriter.Write(newBody)
							return
						}
					}
				}
			}
			w.ResponseWriter.Write(w.body.Bytes())
			return
		}
		c.Next()
	}
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()
	r.Use(MsgpackAndBytesMiddleware())
	svc := &MySmartCityRestService{}
	RegisterSmartCityRestApiHandler(r, svc)
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	fmt.Printf("Go server starting on port %s\n", port)
	http.ListenAndServe(fmt.Sprintf(":%s", port), r)
}
