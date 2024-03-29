// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package protoutil

import (
	"bytes"
	"testing"

	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/stretchr/testify/assert"
)

func TestChunkStreamReadWriter(t *testing.T) {
	b := &testOffsetReadWriter{}

	w, err := NewChunkStreamWriter(b, 1024)
	assert.NoError(t, err)
	r := NewChunkStreamReader(b, 1024)

	src := &networkv1directory.EventBroadcast{
		Events: []*networkv1directory.Event{
			{
				Body: &networkv1directory.Event_Ping_{
					Ping: &networkv1directory.Event_Ping{
						Time: 1257894000000000000,
					},
				},
			},
		},
	}

	for i := 0; i < 3; i++ {
		err = w.Write(src)
		assert.NoError(t, err)
	}

	for i := 0; i < 3; i++ {
		dst := &networkv1directory.EventBroadcast{}
		err = r.Read(dst)
		assert.NoError(t, err)
		assert.Equal(t, src.Events[0].GetPing().Time, dst.Events[0].GetPing().Time)
	}
}

func TestChunkStreamReadWriterLarge(t *testing.T) {
	b := &testOffsetReadWriter{}

	w, err := NewChunkStreamWriter(b, 1024)
	assert.NoError(t, err)
	r := NewChunkStreamReader(b, 1024)

	src := &networkv1directory.EventBroadcast{
		Events: []*networkv1directory.Event{
			{
				Body: &networkv1directory.Event_ListingChange_{
					ListingChange: &networkv1directory.Event_ListingChange{
						Snippet: &networkv1directory.ListingSnippet{
							Thumbnail: &networkv1directory.ListingSnippetImage{
								SourceOneof: &networkv1directory.ListingSnippetImage_Image{
									Image: &image.Image{
										Type: image.ImageType_IMAGE_TYPE_UNDEFINED,
										Data: make([]byte, 32*1024),
									},
								},
							},
						},
					},
				},
			},
		},
	}

	for i := 0; i < 3; i++ {
		err = w.Write(src)
		assert.NoError(t, err)
	}

	for i := 0; i < 3; i++ {
		dst := &networkv1directory.EventBroadcast{}
		err = r.Read(dst)
		assert.NoError(t, err)
	}
}

type testOffsetReadWriter struct {
	bytes.Buffer
}

func (r *testOffsetReadWriter) Offset() uint64 {
	return 0
}

func (r *testOffsetReadWriter) Flush() error {
	return nil
}
