package main

import (
	"context"
	"errors"
	"fmt"
	"net/http/httptest"
	"os"
	"reflect"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type files struct{}

func (*files) GetFile(_ context.Context, req *FilesGetFileRequest) (*FilesGetFileResponse, error) {
	switch req.Id {
	case "cached":
		return nil, &NotModified{Etag: `"v2"`}
	case "empty":
		return nil, &EmptyGone{}
	case "missing":
		return nil, &NotFound{Code: 40401, Msg: "not found"}
	case "reserved":
		return nil, &Reserved{Type_: "conditional", Select_: "retry", TypeID: "raw%2F+"}
	case "stale", "stale-minimal":
		err := &PreconditionFailed{Etag: `"v2"`, RenameAbc: "legacy", SessionId: "renewed", Code: 41201, Msg: "stale revision"}
		if req.Id == "stale" {
			hints, attempts := []string{"reload", "retry"}, []string{"first", "second"}
			retry, detail := "001/;%", "reload before retry"
			err.Hints, err.Attempts, err.RetryToken, err.Detail = &hints, &attempts, &retry, &detail
		}
		return nil, err
	case "framework-typed":
		return nil, xidlgohttp.NewHttpError(412, "framework conflict")
	case "framework":
		return nil, xidlgohttp.NewHttpError(500, "storage unavailable")
	default:
		return &FilesGetFileResponse{Return: FileMeta{Id: req.Id, Etag: "v2"}}, nil
	}
}

type naming struct{}

func (*naming) Read(_ context.Context, req *NamingReadRequest) (*NamingReadResponse, error) {
	if req.Id == "client" {
		return nil, &CollisionFilesClient{Reason: "client name"}
	}
	return nil, &CollisionFramework{Reason: "declared framework"}
}

type scoped struct{}

func (*scoped) Read(_ context.Context, _ *ScopedReadRequest) (*ScopedReadResponse, error) {
	return nil, &LeftFailure{Detail: LeftDetail{Message: "nested exception"}}
}

func require(ok bool, message string) {
	if !ok {
		panic(message)
	}
}

func sdk(base string) {
	ctx := context.Background()
	client := NewFilesClient(base, nil, xidlgohttp.ClientAuth{})
	value, err := client.GetFile(ctx, &FilesGetFileRequest{Id: "ok"})
	require(err == nil && value.Return.Id == "ok", "normal response")
	_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: "cached"})
	var cached *NotModified
	require(errors.As(err, &cached) && cached.Etag == `"v2"`, "typed 304 with ETag")
	for _, full := range []bool{true, false} {
		id := "stale-minimal"
		if full {
			id = "stale"
		}
		_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: id})
		var stale *PreconditionFailed
		require(errors.As(err, &stale), "typed 412")
		require(stale.Etag == `"v2"` && stale.RenameAbc == "legacy" && stale.SessionId == "renewed", "exception HTTP bindings")
		require(stale.Code == 41201 && stale.Msg == "stale revision", "exception body projection")
		if full {
			require(stale.Hints != nil && reflect.DeepEqual(*stale.Hints, []string{"reload", "retry"}), "repeated headers")
			require(stale.Attempts != nil && reflect.DeepEqual(*stale.Attempts, []string{"first", "second"}), "repeated cookies")
			require(stale.RetryToken != nil && *stale.RetryToken == "001/;%", "cookie URI decoding exactly once")
			require(stale.Detail != nil && *stale.Detail == "reload before retry", "optional body field")
		} else {
			require(stale.Hints == nil && stale.Attempts == nil && stale.RetryToken == nil && stale.Detail == nil, "absent optionals")
		}
	}
	_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: "empty"})
	var empty *EmptyGone
	require(errors.As(err, &empty), "bodyless 410")
	_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: "missing"})
	var missing *NotFound
	require(errors.As(err, &missing) && missing.Code == 40401, "typed 404")
	_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: "reserved"})
	var reserved *Reserved
	require(errors.As(err, &reserved) && reserved.Type_ == "conditional" && reserved.Select_ == "retry" && reserved.TypeID == "raw%2F+", "ordinary Go field spelling in exception bindings")
	for _, id := range []string{"framework", "framework-typed"} {
		_, err = client.GetFile(ctx, &FilesGetFileRequest{Id: id})
		var framework *xidlgohttp.HttpError
		require(errors.As(err, &framework), "framework fallback when typed shape does not match")
	}
	names := NewNamingClient(base, nil, xidlgohttp.ClientAuth{})
	_, err = names.Read(ctx, &NamingReadRequest{Id: "client"})
	var collision *CollisionFilesClient
	require(errors.As(err, &collision) && collision.Reason == "client name", "client name collision")
	_, err = names.Read(ctx, &NamingReadRequest{Id: "framework"})
	var declared *CollisionFramework
	require(errors.As(err, &declared) && declared.Reason == "declared framework", "framework name collision")
	_, err = NewScopedClient(base, nil, xidlgohttp.ClientAuth{}).Read(ctx, &ScopedReadRequest{})
	var nested *LeftFailure
	require(errors.As(err, &nested) && nested.Detail.Message == "nested exception", "qualified nested exception")
	fmt.Println("generated client exception round trips passed")
}

func main() {
	router := gin.New()
	RegisterFilesHandler(router, &files{})
	RegisterNamingHandler(router, &naming{})
	RegisterScopedHandler(router, &scoped{})
	server := httptest.NewServer(router)
	sdk(server.URL)
	server.Close()
	if err := router.Run("127.0.0.1:" + os.Getenv("PORT")); err != nil {
		panic(err)
	}
}
