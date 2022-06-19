// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"math/rand"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"

	parser "github.com/MemeLabs/chat-parser"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/sortutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"mvdan.cc/xurls/v2"
)

func newEntityExtractor() *entityExtractor {
	return &entityExtractor{
		parserCtx:         parser.NewParserContext(parser.ParserContextValues{}),
		urls:              xurls.Relaxed(),
		emoji:             compileEmojiRegexp(),
		internalModifiers: syncutil.NewPointer(&[]*chatv1.Modifier{}),
	}
}

// entityExtractor holds active emotes, modifiers, tags and a URL regex
type entityExtractor struct {
	parserCtx         *parser.ParserContext
	urls              *regexp.Regexp
	emoji             *regexp.Regexp
	internalModifiers syncutil.Pointer[[]*chatv1.Modifier]
}

func (x *entityExtractor) ParserContext() *parser.ParserContext {
	return x.parserCtx
}

// SetInternalModifiers updates the rare emote modifiers
func (x *entityExtractor) SetInternalModifiers(v []*chatv1.Modifier) {
	x.internalModifiers.Swap(&v)
}

// Extract splits a message string into it's component entities
func (x *entityExtractor) Extract(msg string) *chatv1.Message_Entities {
	e := &chatv1.Message_Entities{}

	for _, b := range x.urls.FindAllStringSubmatchIndex(msg, -1) {
		url := msg[b[0]:b[1]]
		if b[2] == -1 {
			url = "https://" + url
		}

		e.Links = append(e.Links, &chatv1.Message_Entities_Link{
			Url:    url,
			Bounds: runeBounds(msg, b),
		})
	}

	addEntitiesFromSpan(e, parser.NewParser(x.parserCtx, parser.NewLexer(msg)).ParseMessage())

	for _, b := range x.emoji.FindAllStringIndex(msg, -1) {
		if !inBounds(e.Links, b[0], b[1]) && !inBounds(e.CodeBlocks, b[0], b[1]) {
			e.Emojis = append(e.Emojis, &chatv1.Message_Entities_Emoji{
				Description: emojiDescriptions[msg[b[0]:b[1]]],
				Bounds:      runeBounds(msg, b),
			})
		}
	}

	for _, m := range *x.internalModifiers.Get() {
		if len(e.Emotes) != 0 && rand.Float64() <= m.ProcChance {
			i := rand.Intn(len(e.Emotes))
			e.Emotes[i].Modifiers = append(e.Emotes[i].Modifiers, m.Name)
		}
	}

	return e
}

// recirsively extracts entities
func addEntitiesFromSpan(e *chatv1.Message_Entities, span *parser.Span) {
	switch span.Type {
	case parser.SpanCode:
		e.CodeBlocks = append(e.CodeBlocks, &chatv1.Message_Entities_CodeBlock{
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(span.Pos()), End: uint32(span.End())},
		})
	case parser.SpanSpoiler:
		e.Spoilers = append(e.Spoilers, &chatv1.Message_Entities_Spoiler{
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(span.Pos()), End: uint32(span.End())},
		})
	case parser.SpanGreentext:
		e.GreenText = &chatv1.Message_Entities_GenericEntity{
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(span.Pos()), End: uint32(span.End())},
		}
	case parser.SpanMe:
		e.SelfMessage = &chatv1.Message_Entities_GenericEntity{
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(span.Pos()), End: uint32(span.End())},
		}
	}

	for _, node := range span.Nodes {
		addEntitiesFromNode(e, node)
	}
}

func addEntitiesFromNode(e *chatv1.Message_Entities, node parser.Node) {
	if inBounds(e.Links, node.Pos(), node.End()) {
		return
	}

	switch n := node.(type) {
	case *parser.Emote:
		e.Emotes = append(e.Emotes, &chatv1.Message_Entities_Emote{
			Name:      n.Name,
			Modifiers: n.Modifiers,
			Bounds:    &chatv1.Message_Entities_Bounds{Start: uint32(n.Pos()), End: uint32(n.End())},
		})
	case *parser.Nick:
		e.Nicks = append(e.Nicks, &chatv1.Message_Entities_Nick{
			Nick:    n.Nick,
			Bounds:  &chatv1.Message_Entities_Bounds{Start: uint32(n.Pos()), End: uint32(n.End())},
			PeerKey: n.Meta.([]byte),
		})
	case *parser.Tag:
		e.Tags = append(e.Tags, &chatv1.Message_Entities_Tag{
			Name:   n.Name,
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(n.Pos()), End: uint32(n.End())},
		})
	case *parser.Span:
		addEntitiesFromSpan(e, n)
	}
}

type boundsGetter interface {
	GetBounds() *chatv1.Message_Entities_Bounds
}

func inBounds[T boundsGetter](ls []T, start, end int) bool {
	for _, l := range ls {
		b := l.GetBounds()
		if b.Start <= uint32(start) && b.End >= uint32(end) {
			return true
		}
	}
	return false
}

func runeBounds(msg string, b []int) *chatv1.Message_Entities_Bounds {
	off := utf8.RuneCountInString(msg[:b[0]])
	width := utf8.RuneCountInString(msg[b[0]:b[1]])

	return &chatv1.Message_Entities_Bounds{
		Start: uint32(off),
		End:   uint32(off + width),
	}
}

func compileEmojiRegexp() *regexp.Regexp {
	glyphs := make([]string, 0, len(emojiDescriptions))
	for c := range emojiDescriptions {
		glyphs = append(glyphs, regexp.QuoteMeta(c))
	}
	sort.Sort(sortutil.OrderedSlice[string](glyphs))

	re := regexp.MustCompile(strings.Join(glyphs, "|"))
	re.Longest()
	return re
}
