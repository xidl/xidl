package http_stream

import (
	"context"
	"io"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type testHttpStreamService struct{}

func (testHttpStreamService) Alerts(
	ctx context.Context,
	req *HttpStreamApiAlertsRequest,
	stream xidlgohttp.ServerStreamWriter[string],
) error {
	if _, ok := xidlgohttp.BasicAuthFromContext(ctx); !ok {
		tpanic("basic auth missing from context")
	}
	if err := stream.Write(req.District + ":ALERT:1"); err != nil {
		return err
	}
	return stream.Write(req.District + ":ALERT:2")
}

func (testHttpStreamService) UploadAsset(
	ctx context.Context,
	stream *xidlgohttp.ClientStreamReader[HttpStreamApiUploadAssetRequest],
) (*HttpStreamApiUploadAssetResponse, error) {
	if _, ok := xidlgohttp.BearerFromContext(ctx); !ok {
		tpanic("bearer auth missing from context")
	}
	total := 0
	assetID := ""
	for {
		item, err := stream.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
		assetID = item.AssetId
		total += len(item.Chunk)
	}
	return &HttpStreamApiUploadAssetResponse{
		Return: "uploaded:" + assetID + ":" + xidlgohttp.FormatInt64(int64(total)),
	}, nil
}

func TestHttpStream(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	RegisterHttpStreamApiHandler(r, testHttpStreamService{})
	server := httptest.NewServer(r)
	defer server.Close()

	client := NewHttpStreamApiClient(server.URL, server.Client(), xidlgohttp.ClientAuth{
		Basic: &xidlgohttp.BasicAuth{
			Username: "user",
			Password: "pass",
		},
		Bearer: "token-1",
	})

	alerts, err := client.Alerts(context.Background(), &HttpStreamApiAlertsRequest{
		District: "pudong",
	})
	if err != nil {
		t.Fatalf("Alerts failed: %v", err)
	}
	defer alerts.Close()

	first, err := alerts.Read()
	if err != nil {
		t.Fatalf("read first alert: %v", err)
	}
	second, err := alerts.Read()
	if err != nil {
		t.Fatalf("read second alert: %v", err)
	}
	if first != "pudong:ALERT:1" || second != "pudong:ALERT:2" {
		t.Fatalf("unexpected alerts: %q %q", first, second)
	}

	upload, err := client.UploadAsset(context.Background())
	if err != nil {
		t.Fatalf("UploadAsset failed: %v", err)
	}
	if err := upload.Write(HttpStreamApiUploadAssetRequest{
		AssetId: "asset-1",
		Chunk:   []byte{1, 2, 3},
	}); err != nil {
		t.Fatalf("write first chunk: %v", err)
	}
	if err := upload.Write(HttpStreamApiUploadAssetRequest{
		AssetId: "asset-1",
		Chunk:   []byte{4, 5},
	}); err != nil {
		t.Fatalf("write second chunk: %v", err)
	}
	result, err := upload.Close()
	if err != nil {
		t.Fatalf("close upload: %v", err)
	}
	if result.Return != "uploaded:asset-1:5" {
		t.Fatalf("unexpected upload result: %q", result.Return)
	}
}

func tpanic(message string) {
	panic(message)
}
