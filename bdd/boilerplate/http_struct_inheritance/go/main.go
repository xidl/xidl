package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type MyAccounts struct{}

func (s *MyAccounts) Participant(ctx context.Context, req *AccountsParticipantRequest) (*AccountsParticipantResponse, error) {
	return &AccountsParticipantResponse{
		Return: Participant{
			UserRef: UserRef{Id: "u-1"},
			Role:    "admin",
		},
	}, nil
}

func (s *MyAccounts) Echo(ctx context.Context, req *AccountsEchoRequest) (*AccountsEchoResponse, error) {
	return &AccountsEchoResponse{Return: req.Participant}, nil
}

func main() {
	r := gin.Default()
	RegisterAccountsHandler(r, &MyAccounts{})
	http.ListenAndServe(fmt.Sprintf(":%s", os.Getenv("PORT")), r)
}
