package directory

import (
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"go.uber.org/zap/zapcore"
)

func marshalListingLogObject(l *networkv1directory.Listing, e zapcore.ObjectEncoder) {
	switch c := l.Content.(type) {
	case *networkv1directory.Listing_Media_:
		e.AddString("type", "media")
		e.AddString("uri", c.Media.SwarmUri)
	case *networkv1directory.Listing_Service_:
		e.AddString("type", "service")
		e.AddString("service", c.Service.Type)
	case *networkv1directory.Listing_Embed_:
		e.AddString("type", "embed")
		e.AddString("service", embedServiceName(c.Embed.Service))
		e.AddString("id", c.Embed.GetId())
	case *networkv1directory.Listing_Chat_:
		e.AddString("type", "chat")
		e.AddBinary("key", c.Chat.Key)
		e.AddString("name", c.Chat.Name)
	}
}
