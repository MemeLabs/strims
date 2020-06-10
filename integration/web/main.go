package main

import (
	"context"
	"log"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

func main() {
	c, err := driver.NewHeadless()
	if err != nil {
		log.Fatal(err)
	}
	defer c.Close()

	req := &pb.CreateProfileRequest{
		Name:     "test",
		Password: "test",
	}
	res := &pb.CreateProfileResponse{}
	if err := c.CallUnary(context.Background(), "createProfile", req, res); err != nil {
		log.Fatal(err)
	}

	log.Println(res)
}
