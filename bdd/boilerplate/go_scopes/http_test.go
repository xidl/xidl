package api

import (
	"context"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type entries struct {
	calls  int
	stored map[string]LedgerEntriesEnvelope
}

func (service *entries) Save(_ context.Context, request *LedgerEntriesSaveRequest) (*LedgerEntriesSaveResponse, error) {
	service.calls++
	if request.Value.Entry.Id != request.Id {
		return nil, &LedgerFailure{Detail: LedgerDetail{Message: "entry identity mismatch"}}
	}
	if request.Value.Entry.State == LedgerStateLocked {
		return nil, &LedgerFailure{Detail: LedgerDetail{Message: "entry is locked"}}
	}
	service.stored[request.Id] = request.Value
	return &LedgerEntriesSaveResponse{Return: request.Value}, nil
}

func TestScopedSDKAndServerUseTheSameContract(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	service := &entries{stored: make(map[string]LedgerEntriesEnvelope)}
	RegisterLedgerEntriesHandler(router, service)
	server := httptest.NewServer(router)
	defer server.Close()
	client := NewLedgerEntriesClient(server.URL, server.Client(), xidlgohttp.ClientAuth{})
	value := LedgerEntriesEnvelope{Entry: entry(), Detail: LedgerEntriesView{Message: "saved"}}
	request := &LedgerEntriesSaveRequest{Id: value.Entry.Id, Value: value}
	for range 2 {
		response, err := client.Save(context.Background(), request)
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(response.Return, value) || !reflect.DeepEqual(service.stored[request.Id], value) {
			t.Fatal("SDK, server or storage changed scoped type identity")
		}
	}
	if len(service.stored) != 1 {
		t.Fatal("repeated save created another entry")
	}
	for _, change := range []struct {
		id      string
		state   LedgerState
		message string
	}{
		{"another-id", LedgerStateOpen, "entry identity mismatch"},
		{request.Id, LedgerStateLocked, "entry is locked"},
	} {
		rejected := *request
		rejected.Id, rejected.Value.Entry.State = change.id, change.state
		_, err := client.Save(context.Background(), &rejected)
		var failure *LedgerFailure
		if !errors.As(err, &failure) || failure.Detail.Message != change.message {
			t.Fatalf("scoped exception did not round trip: %v", err)
		}
		if !reflect.DeepEqual(service.stored[request.Id], value) {
			t.Fatal("rejected change mutated the stored entry")
		}
	}
	calls := service.calls
	response, err := server.Client().Post(server.URL+"/entries/entry-1", "application/json",
		strings.NewReader(`{"detail":{"message":7}}`))
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != http.StatusBadRequest || service.calls != calls {
		t.Fatalf("invalid scoped input reached business logic: %d %s", response.StatusCode, body)
	}
}
