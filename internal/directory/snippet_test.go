package directory

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/image"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/stretchr/testify/assert"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/wrapperspb"
)

func getTestSnippets() (a, b *networkv1directory.ListingSnippet, k *key.Key) {
	k, _ = dao.GenerateKey()

	a = &networkv1directory.ListingSnippet{
		Title:       "test a",
		Description: "some description",
		Tags:        []string{"a", "b", "c"},
		Category:    "",
		ChannelName: "test channel name",
		ViewerCount: uint64(0),
		Live:        false,
		IsMature:    true,
		VideoHeight: 1080,
		VideoWidth:  1920,
		Thumbnail: &networkv1directory.ListingSnippetImage{
			SourceOneof: &networkv1directory.ListingSnippetImage_Image{
				Image: &image.Image{
					Type: image.ImageType_IMAGE_TYPE_JPEG,
					Data: make([]byte, 1024),
				},
			},
		},
		ChannelLogo: &networkv1directory.ListingSnippetImage{
			SourceOneof: &networkv1directory.ListingSnippetImage_Url{
				Url: "https://test.com/a.png",
			},
		},
	}
	dao.SignMessage(a, k)

	b = &networkv1directory.ListingSnippet{
		Title:       "test b",
		Description: "some description",
		Tags:        []string{},
		Category:    "test category",
		ChannelName: "",
		ViewerCount: uint64(10),
		Live:        true,
		IsMature:    false,
		Thumbnail:   nil,
		ChannelLogo: &networkv1directory.ListingSnippetImage{
			SourceOneof: &networkv1directory.ListingSnippetImage_Url{
				Url: "https://test.com/b.png",
			},
		},
	}
	dao.SignMessage(b, k)

	return
}

func TestDiffSnippets(t *testing.T) {
	a, b, _ := getTestSnippets()
	delta := diffSnippets(a, b)

	assert.True(t, proto.Equal(delta, &networkv1directory.ListingSnippetDelta{
		Title:       &wrapperspb.StringValue{Value: b.Title},
		Category:    &wrapperspb.StringValue{Value: b.Category},
		ChannelName: &wrapperspb.StringValue{Value: b.ChannelName},
		ViewerCount: &wrapperspb.UInt64Value{Value: b.ViewerCount},
		Live:        &wrapperspb.BoolValue{Value: b.Live},
		IsMature:    &wrapperspb.BoolValue{Value: b.IsMature},
		VideoHeight: &wrapperspb.UInt32Value{Value: b.VideoHeight},
		VideoWidth:  &wrapperspb.UInt32Value{Value: b.VideoWidth},
		TagsOneof: &networkv1directory.ListingSnippetDelta_Tags_{
			Tags: &networkv1directory.ListingSnippetDelta_Tags{},
		},
		ThumbnailOneof: &networkv1directory.ListingSnippetDelta_Thumbnail{
			Thumbnail: nil,
		},
		ChannelLogoOneof: &networkv1directory.ListingSnippetDelta_ChannelLogo{
			ChannelLogo: &networkv1directory.ListingSnippetImage{
				SourceOneof: &networkv1directory.ListingSnippetImage_Url{
					Url: b.GetChannelLogo().GetUrl(),
				},
			},
		},
		Signature: &wrapperspb.BytesValue{Value: b.Signature},
	}))
}

func TestIsNilSnippetDelta(t *testing.T) {
	assert.True(t, isNilSnippetDelta(&networkv1directory.ListingSnippetDelta{}))
}

func TestDiffSnippetsWithEqualValues(t *testing.T) {
	a, _, _ := getTestSnippets()
	assert.True(t, isNilSnippetDelta(diffSnippets(a, a)))
}

func TestMergeSnippet(t *testing.T) {
	a, b, _ := getTestSnippets()
	c := proto.Clone(a).(*networkv1directory.ListingSnippet)
	mergeSnippet(c, diffSnippets(a, b))
	assert.True(t, proto.Equal(c, b))
}

func TestMergeSnippetCopy(t *testing.T) {
	_, b, _ := getTestSnippets()
	c := &networkv1directory.ListingSnippet{}
	mergeSnippet(c, diffSnippets(c, b))
	assert.True(t, proto.Equal(c, b))
}
