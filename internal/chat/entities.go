package chat

import (
	"math/rand"
	"regexp"

	parser "github.com/MemeLabs/chat-parser"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"mvdan.cc/xurls/v2"
)

func newEntityExtractor() *entityExtractor {
	return &entityExtractor{
		parserCtx: parser.NewParserContext(parser.ParserContextValues{}),
		urls:      xurls.Relaxed(),
	}
}

// entityExtractor holds active emotes, modifiers, tags and a URL regex
type entityExtractor struct {
	parserCtx *parser.ParserContext
	urls      *regexp.Regexp
	rareRate  float64
}

// AddNick adds a nick to the EntityExtractor
func (x *entityExtractor) AddNick(nick string) {
	x.parserCtx.Nicks.Insert([]rune(nick))
}

// RemoveNick removes a nick from the EntityExtractor
func (x *entityExtractor) RemoveNick(nick string) {
	x.parserCtx.Nicks.Remove([]rune(nick))
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
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(b[0]), End: uint32(b[1])},
		})
	}

	addEntitiesFromSpan(e, parser.NewParser(x.parserCtx, parser.NewLexer(msg)).ParseMessage())

	if len(e.Emotes) != 0 && rand.Float64() <= x.rareRate {
		i := rand.Intn(len(e.Emotes))
		e.Emotes[i].Modifiers = append(e.Emotes[i].Modifiers, "rare")
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
	for _, l := range e.Links {
		if l.Bounds.Start <= uint32(node.Pos()) && l.Bounds.End >= uint32(node.End()) {
			// skip node if we are in a link span
			return
		}
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
			Nick:   n.Nick,
			Bounds: &chatv1.Message_Entities_Bounds{Start: uint32(n.Pos()), End: uint32(n.End())},
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
