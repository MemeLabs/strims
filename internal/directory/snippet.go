package directory

import (
	"bytes"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"golang.org/x/exp/slices"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/wrapperspb"
)

var nilSnippet = &networkv1directory.ListingSnippet{}
var nilSnippetDelta = &networkv1directory.ListingSnippetDelta{}

func diffSnippets(a, b *networkv1directory.ListingSnippet) *networkv1directory.ListingSnippetDelta {
	delta := &networkv1directory.ListingSnippetDelta{}

	if a.Title != b.Title {
		delta.Title = &wrapperspb.StringValue{Value: b.Title}
	}
	if a.Description != b.Description {
		delta.Description = &wrapperspb.StringValue{Value: b.Description}
	}
	if !slices.Equal(a.Tags, b.Tags) {
		delta.TagsOneof = &networkv1directory.ListingSnippetDelta_Tags_{Tags: &networkv1directory.ListingSnippetDelta_Tags{Tags: b.Tags}}
	}
	if a.Category != b.Category {
		delta.Category = &wrapperspb.StringValue{Value: b.Category}
	}
	if a.ChannelName != b.ChannelName {
		delta.ChannelName = &wrapperspb.StringValue{Value: b.ChannelName}
	}
	if a.ViewerCount != b.ViewerCount {
		delta.ViewerCount = &wrapperspb.UInt64Value{Value: b.ViewerCount}
	}
	if a.Live != b.Live {
		delta.Live = &wrapperspb.BoolValue{Value: b.Live}
	}
	if a.IsMature != b.IsMature {
		delta.IsMature = &wrapperspb.BoolValue{Value: b.IsMature}
	}
	if a.VideoHeight != b.VideoHeight {
		delta.VideoHeight = &wrapperspb.UInt32Value{Value: b.VideoHeight}
	}
	if a.VideoWidth != b.VideoWidth {
		delta.VideoWidth = &wrapperspb.UInt32Value{Value: b.VideoWidth}
	}
	if a.ThemeColor != b.ThemeColor {
		delta.ThemeColor = &wrapperspb.UInt32Value{Value: b.ThemeColor}
	}
	if !proto.Equal(a.ChannelLogo, b.ChannelLogo) {
		delta.ChannelLogoOneof = &networkv1directory.ListingSnippetDelta_ChannelLogo{ChannelLogo: b.ChannelLogo}
	}
	if !proto.Equal(a.Thumbnail, b.Thumbnail) {
		delta.ThumbnailOneof = &networkv1directory.ListingSnippetDelta_Thumbnail{Thumbnail: b.Thumbnail}
	}
	if !bytes.Equal(a.Key, b.Key) {
		key := make([]byte, len(b.Key))
		copy(key, b.Key)
		delta.Key = &wrapperspb.BytesValue{Value: key}
	}
	if !bytes.Equal(a.Signature, b.Signature) {
		signature := make([]byte, len(b.Signature))
		copy(signature, b.Signature)
		delta.Signature = &wrapperspb.BytesValue{Value: signature}
	}

	return delta
}

func mergeSnippet(snippet *networkv1directory.ListingSnippet, delta *networkv1directory.ListingSnippetDelta) {
	if delta.Title != nil {
		snippet.Title = delta.Title.Value
	}
	if delta.Description != nil {
		snippet.Description = delta.Description.Value
	}
	if tags := delta.GetTags(); tags != nil {
		snippet.Tags = tags.Tags
	}
	if delta.Category != nil {
		snippet.Category = delta.Category.Value
	}
	if delta.ChannelName != nil {
		snippet.ChannelName = delta.ChannelName.Value
	}
	if delta.ViewerCount != nil {
		snippet.ViewerCount = delta.ViewerCount.Value
	}
	if delta.Live != nil {
		snippet.Live = delta.Live.Value
	}
	if delta.IsMature != nil {
		snippet.IsMature = delta.IsMature.Value
	}
	if delta.VideoHeight != nil {
		snippet.VideoHeight = delta.VideoHeight.Value
	}
	if delta.VideoWidth != nil {
		snippet.VideoWidth = delta.VideoWidth.Value
	}
	if delta.ThemeColor != nil {
		snippet.ThemeColor = delta.ThemeColor.Value
	}
	if delta.ThumbnailOneof != nil {
		snippet.Thumbnail = delta.GetThumbnail()
	}
	if delta.ChannelLogoOneof != nil {
		snippet.ChannelLogo = delta.GetChannelLogo()
	}
	if delta.Key != nil {
		snippet.Key = delta.Key.Value
	}
	if delta.Signature != nil {
		snippet.Signature = delta.Signature.Value
	}
}

func isNilSnippetDelta(delta *networkv1directory.ListingSnippetDelta) bool {
	return proto.Equal(nilSnippetDelta, delta)
}
