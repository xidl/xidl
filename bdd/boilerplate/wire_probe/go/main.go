package main

import (
	"context"
	"errors"
	"fmt"
	"github.com/gin-gonic/gin"
	xidlgo "github.com/xidl/xidl/golang/xidl-go"
	xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
	"io"
	"os"
)

type store struct{}

func item() CatalogItem {
	return CatalogItem{Sku: "sku-雪", InternalNote: "private", Availability: CatalogAvailabilityInStock,
		StockGrid: [2][3]int32{{0, 1, 2}, {3, 4, 5}}, Thumbnail: []byte{0, 127, 255}, Revision: 9007199254740991,
		Initial: xidlgo.Char('A'), Mark: xidlgo.Char('🚀'), Letters: []xidlgo.Char{'A', '雪', '🚀'}}
}
func conflict() *Conflict { return &Conflict{Sku: "locked", Reason: "inventory locked"} }
func (*store) Echo(_ context.Context, req *StoreEchoRequest) (*StoreEchoResponse, error) {
	if req.Item.Sku == "locked" {
		return nil, conflict()
	}
	return &StoreEchoResponse{Return: req.Item}, nil
}
func (*store) Apply(_ context.Context, req *StoreApplyRequest) (*StoreApplyResponse, error) {
	return &StoreApplyResponse{Return: req.Change}, nil
}
func (*store) Label(_ context.Context, req *StoreLabelRequest) (*StoreLabelResponse, error) {
	return &StoreLabelResponse{Return: req.Change}, nil
}
func (*store) Bindings(_ context.Context, req *StoreBindingsRequest) (*StoreBindingsResponse, error) {
	return &StoreBindingsResponse{Return: fmt.Sprintf("%d:%c:%s", req.Count, req.Initial, req.Session), CountReply: req.Count, Receipt: req.Session}, nil
}
func (*store) Feed(_ context.Context, req *StoreFeedRequest, stream xidlgohttp.ServerStreamWriter[CatalogChange]) error {
	if req.State == "locked" {
		return conflict()
	}
	if err := stream.Write(NewCatalogChangeItem(item())); err != nil {
		return err
	}
	if req.State == "partial" {
		return conflict()
	}
	return stream.Write(NewCatalogChangeReason("retired"))
}
func require(ok bool, reason string) {
	if !ok {
		panic(reason)
	}
}
func sdk(base string) {
	client := NewStoreClient(base, nil, xidlgohttp.ClientAuth{})
	ctx := context.Background()
	echoed, err := client.Echo(ctx, &StoreEchoRequest{Item: item()})
	if err != nil {
		panic(err)
	}
	value := echoed.Return
	require(value.Sku == "sku-雪" && value.InternalNote == "" && value.Memo == nil, "field naming or presence")
	require(value.Revision == 9007199254740991 && value.Mark == '🚀' && value.Thumbnail[2] == 255 && value.StockGrid[1][2] == 5, "typed item payload")
	locked := item()
	locked.Sku = "locked"
	_, err = client.Echo(ctx, &StoreEchoRequest{Item: locked})
	var typed *Conflict
	require(errors.As(err, &typed) && typed.Sku == "locked", "declared unary error")
	changed, err := client.Apply(ctx, &StoreApplyRequest{Change: NewCatalogChangeItem(item())})
	if err != nil {
		panic(err)
	}
	payload, ok := changed.Return.AsItem()
	require(ok && payload.Mark == '🚀', "typed union object")
	batch, err := client.Apply(ctx, &StoreApplyRequest{Change: NewCatalogChangeBatch([]CatalogItem{item()})})
	if err != nil {
		panic(err)
	}
	items, ok := batch.Return.AsBatch()
	require(ok && items[0].Sku == "sku-雪", "typed union collection")
	retired, err := client.Apply(ctx, &StoreApplyRequest{Change: NewCatalogChangeReason("retired")})
	if err != nil {
		panic(err)
	}
	reason, ok := retired.Return.AsReason()
	require(ok && reason == "retired", "typed default union")
	label, err := client.Label(ctx, &StoreLabelRequest{Change: NewCatalogLabelChangeReason("retired")})
	if err != nil {
		panic(err)
	}
	reason, ok = label.Return.AsReason()
	require(ok && reason == "retired", "explicit case sharing a default branch")
	bound, err := client.Bindings(ctx, &StoreBindingsRequest{Count: 7, Initial: 'A', Session: "%2F+ %"})
	if err != nil {
		panic(err)
	}
	require(bound.Return == "7:A:%2F+ %" && bound.CountReply == 7 && bound.Receipt == "%2F+ %", "text transport round trip")
	for _, state := range []string{"ready", "partial"} {
		stream, err := client.Feed(ctx, &StoreFeedRequest{State: state})
		if err != nil {
			panic(err)
		}
		first, err := stream.Read()
		if err != nil {
			panic(err)
		}
		_, ok := first.AsItem()
		require(ok, "typed stream first item")
		last, err := stream.Read()
		if state == "partial" {
			require(errors.As(err, &typed), "declared stream error")
		} else {
			reason, ok := last.AsReason()
			require(err == nil && ok && reason == "retired", "stream default item")
		}
		_, err = stream.Read()
		require(errors.Is(err, io.EOF), "terminal stream outcome")
		if err := stream.Close(); err != nil {
			panic(err)
		}
	}
}
func main() {
	if len(os.Args) > 1 {
		sdk(os.Args[1])
		return
	}
	router := gin.New()
	RegisterStoreHandler(router, &store{})
	if err := router.Run("127.0.0.1:" + os.Getenv("PORT")); err != nil {
		panic(err)
	}
}
