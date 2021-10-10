package directory

import (
	"io/ioutil"
	"log"
	"net/http"
)

func testLoadImage() {
	res, err := http.Get("https://thumbnail.angelthump.com/thumbnails/spf1general.jpeg")
	if err != nil {
		log.Println(err)
		return
	}
	b, err := ioutil.ReadAll(res.Body)
	if err != nil {
		log.Println(err)
		return
	}
	log.Println(">>> bytes of image", len(b))
}
