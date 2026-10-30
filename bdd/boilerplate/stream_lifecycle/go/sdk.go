package main

import (
	"context"
	"errors"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
	"io"
)

func require(ok bool) {
	if !ok {
		panic("stream lifecycle contract failed")
	}
}
func runSDK(base string) {
	for _, body := range []string{`{}`, `{"detail":null}`} {
		failure := decodeLedgerExportRowsFailure(xidlgohttp.StreamFailure{Status: 409, Body: []byte(body)})
		var runtime *xidlgohttp.HttpError
		require(errors.As(failure, &runtime))
	}
	client := NewLedgerClient(base, nil, xidlgohttp.ClientAuth{})
	ctx := context.Background()
	reader, err := client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "ready"})
	require(err == nil)
	first, err := reader.Read()
	require(err == nil && first.Quantity == 0)
	second, err := reader.Read()
	require(err == nil && second.Id == "雪" && second.Quantity == 2)
	_, err = reader.Read()
	require(errors.Is(err, io.EOF) && reader.Outcome() == xidlgohttp.StreamCompleted)
	var ex *Conflict
	_, err = client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "locked"})
	require(errors.As(err, &ex) && ex.Detail.Job == "locked")
	var retired *Retired
	_, err = client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "retired"})
	require(errors.As(err, &retired))
	partial, err := client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "partial"})
	require(err == nil)
	first, err = partial.Read()
	require(err == nil && first.Quantity == 0)
	_, err = partial.Read()
	require(errors.As(err, &ex) && ex.Detail.Job == "partial" && partial.Outcome() == xidlgohttp.StreamFailed)
	_ = partial.Close()
	_ = partial.Close()
	require(partial.Outcome() == xidlgohttp.StreamFailed)
	empty, err := client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "empty"})
	require(err == nil)
	_, err = empty.Read()
	require(errors.Is(err, io.EOF) && empty.Outcome() == xidlgohttp.StreamCompleted)
	cancelled, err := client.ExportRows(ctx, &LedgerExportRowsRequest{Job: "ready"})
	require(err == nil)
	_, err = cancelled.Read()
	require(err == nil)
	_ = cancelled.Close()
	require(cancelled.Outcome() == xidlgohttp.StreamCancelled)
	upload, err := client.ImportRows(ctx)
	require(err == nil)
	require(upload.Write(LedgerImportRowsRequest{Row: ExportRow{Id: "zero", Quantity: 0}}) == nil)
	require(upload.Write(LedgerImportRowsRequest{Row: ExportRow{Id: "雪", Quantity: 2}}) == nil)
	result, err := upload.Close()
	require(err == nil && result.Return.Accepted == 2 && result.Return.Total == 2)
	upload, err = client.ImportRows(ctx)
	require(err == nil)
	for range 2 {
		require(upload.Write(LedgerImportRowsRequest{Row: ExportRow{Id: "duplicate", Quantity: 1}}) == nil)
	}
	_, err = upload.Close()
	require(errors.As(err, &ex) && ex.Detail.Job == "duplicate")
}
