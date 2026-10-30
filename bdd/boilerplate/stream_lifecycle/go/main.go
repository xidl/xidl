package main

import (
	"context"
	"errors"
	"io"
	"os"

	"github.com/gin-gonic/gin"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
)

type ledger struct{}

func conflict(job, reason string) error {
	return &Conflict{Detail: ConflictDetail{Job: job, Reason: reason}}
}
func (*ledger) ExportRows(ctx context.Context, req *LedgerExportRowsRequest, stream xidlgohttp.ServerStreamWriter[ExportRow]) error {
	switch req.Job {
	case "locked":
		return conflict(req.Job, "locked")
	case "retired":
		return &Retired{}
	case "empty":
		return nil
	}
	if err := stream.Write(ExportRow{Id: "zero", Quantity: 0}); err != nil {
		return err
	}
	if req.Job == "partial" {
		return conflict(req.Job, "locked")
	}
	return stream.Write(ExportRow{Id: "雪", Quantity: 2})
}
func (*ledger) ImportRows(ctx context.Context, input *xidlgohttp.ClientStreamReader[LedgerImportRowsRequest]) (*LedgerImportRowsResponse, error) {
	ids := map[string]bool{}
	total := int32(0)
	for {
		value, err := input.Read()
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return nil, xidlgohttp.NewHttpError(400, err.Error())
		}
		row := value.Row
		if ids[row.Id] || row.Quantity < 0 {
			return nil, conflict(row.Id, "duplicate or negative quantity")
		}
		ids[row.Id] = true
		total += row.Quantity
	}
	return &LedgerImportRowsResponse{Return: ImportResult{Accepted: int32(len(ids)), Total: total}}, nil
}
func main() {
	if len(os.Args) > 1 {
		runSDK(os.Args[1])
		return
	}
	router := gin.New()
	RegisterLedgerHandler(router, &ledger{})
	if err := router.Run("127.0.0.1:" + os.Getenv("PORT")); err != nil {
		panic(err)
	}
}
