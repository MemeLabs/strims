package service

// Ripped from the old chat with some parts removed

import (
	"regexp"

	parser "github.com/MemeLabs/chat-parser"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"mvdan.cc/xurls/v2"
)

var entities *EntityExtractor

func init() {
	// TODO: remove hardcoded values
	entities = &EntityExtractor{
		parserCtx: parser.NewParserContext(parser.ParserContextValues{
			Emotes:         []string{"INFESTOR", "FIDGETLOL", "Hhhehhehe", "GameOfThrows", "Abathur", "LUL", "SURPRISE", "NoTears", "OverRustle", "DuckerZ", "Kappa", "Klappa", "DappaKappa", "BibleThump", "AngelThump", "BasedGod", "OhKrappa", "SoDoge", "WhoahDude", "MotherFuckinGame", "DaFeels", "UWOTM8", "DatGeoff", "FerretLOL", "Sippy", "Nappa", "DAFUK", "HEADSHOT", "DANKMEMES", "MLADY", "MASTERB8", "NOTMYTEMPO", "LeRuse", "YEE", "SWEATY", "PEPE", "SpookerZ", "WEEWOO", "ASLAN", "TRUMPED", "BASEDWATM8", "BERN", "Hmmm", "PepoThink", "FeelsAmazingMan", "FeelsBadMan", "FeelsGoodMan", "OhMyDog", "Wowee", "haHAA", "POTATO", "NOBULLY", "gachiGASM", "REE", "monkaS", "RaveDoge", "CuckCrab", "MiyanoHype", "ECH", "NiceMeMe", "ITSRAWWW", "Riperino", "4Head", "BabyRage", "Kreygasm", "SMOrc", "NotLikeThis", "POGGERS", "AYAYA", "PepOk", "PepoComfy", "PepoWant", "PepeHands", "BOGGED", "ComfyApe", "ApeHands", "OMEGALUL", "COGGERS", "PepoWant", "Clap", "FeelsWeirdMan", "monkaMEGA", "ComfyDog", "GIMI", "MOOBERS", "PepoBan", "ComfyAYA", "ComfyFerret", "BOOMER", "ZOOMER", "SOY", "FeelsPepoMan", "ComfyCat", "ComfyPOTATO", "SUGOI", "DJPepo", "CampFire", "ComfyYEE", "weSmart", "PepoG", "OBJECTION", "ComfyWeird", "umaruCry", "OsKrappa", "monkaHmm", "PepoHmm", "PepeComfy", "SUGOwO", "EZ", "Pepega", "shyLurk", "FeelsOkayMan", "POKE", "PepoDance", "ORDAH", "SPY", "PepoGood", "PepeJam", "LAG", "billyWeird", "SOTRIGGERED", "OnlyPretending", "cmonBruh", "VroomVroom", "mikuDance", "WAG", "PepoFight", "NeneLaugh", "PepeLaugh", "PeepoS", "SLEEPY", "GODMAN", "NOM", "FeelsDumbMan", "SEMPAI", "OSTRIGGERED", "MiyanoBird", "KING", "PIKOHH", "PepoPirate", "PepeMods", "OhISee", "WeirdChamp", "RedCard", "illyaTriggered", "SadBenis", "PeepoHappy", "ComfyWAG", "MiyanoComfy", "sataniaLUL", "DELUSIONAL", "GREED", "AYAWeird", "FeelsCountryMan", "SNAP", "PeepoRiot", "HiHi", "ComfyFeels", "MiyanoSip", "PeepoWeird", "JimFace", "HACKER", "monkaVirus", "DOUBT", "KEKW", "SHOCK", "DOIT", "GODWOMAN", "POGGIES", "SHRUG", "POGOI", "PepoSleep"},
			Nicks:          []string{},
			Tags:           []string{"nsfw", "weeb", "nsfl", "loud"},
			EmoteModifiers: []string{"mirror", "flip", "rain", "snow", "rustle", "worth", "love", "spin", "wide", "lag", "hyper"},
		}),
		urls: xurls.Relaxed(),
	}
}

// EntityExtractor holds active emotes, modifiers, tags and a URL regex
type EntityExtractor struct {
	parserCtx *parser.ParserContext
	urls      *regexp.Regexp
}

// AddNick adds a nick to the EntityExtractor
func (x *EntityExtractor) AddNick(nick string) {
	x.parserCtx.Nicks.Insert([]rune(nick))
}

// RemoveNick removes a nick from the EntityExtractor
func (x *EntityExtractor) RemoveNick(nick string) {
	x.parserCtx.Nicks.Remove([]rune(nick))
}

// Extract splits a message string into it's component entities
func (x *EntityExtractor) Extract(msg string) *pb.MessageEntities {
	e := &pb.MessageEntities{}

	for _, b := range x.urls.FindAllStringIndex(msg, -1) {
		e.Links = append(e.Links, &pb.Link{
			Url:    msg[b[0]:b[1]],
			Bounds: &pb.Bounds{Start: int64(b[0]), End: int64(b[1])},
		})
	}

	addEntitiesFromSpan(e, parser.NewParser(x.parserCtx, parser.NewLexer(msg)).ParseMessage())

	return e
}

// recirsively extracts entities
func addEntitiesFromSpan(e *pb.MessageEntities, span *parser.Span) {
	switch span.Type {
	case parser.SpanCode:
		e.CodeBlocks = append(e.CodeBlocks, &pb.CodeBlock{
			Bounds: &pb.Bounds{Start: int64(span.Pos()), End: int64(span.End())},
		})
	case parser.SpanSpoiler:
		e.Spoilers = append(e.Spoilers, &pb.Spoiler{
			Bounds: &pb.Bounds{Start: int64(span.Pos()), End: int64(span.End())},
		})
	case parser.SpanGreentext:
		e.GreenText = &pb.GenericEntity{
			Bounds: &pb.Bounds{Start: int64(span.Pos()), End: int64(span.End())},
		}
	case parser.SpanMe:
		e.SelfMessage = &pb.GenericEntity{
			Bounds: &pb.Bounds{Start: int64(span.Pos()), End: int64(span.End())},
		}
	}

	for _, node := range span.Nodes {
		addEntitiesFromNode(e, node)
	}
}

func addEntitiesFromNode(e *pb.MessageEntities, node parser.Node) {
	for _, l := range e.Links {
		if l.Bounds.Start <= int64(node.Pos()) && l.Bounds.End >= int64(node.End()) {
			// skip node if we are in a link span
			return
		}
	}

	switch n := node.(type) {
	case *parser.Emote:
		e.Emotes = append(e.Emotes, &pb.Emote{
			Name:      n.Name,
			Modifiers: n.Modifiers,
			Bounds:    &pb.Bounds{Start: int64(n.Pos()), End: int64(n.End())},
		})
	case *parser.Nick:
		e.Nicks = append(e.Nicks, &pb.Nick{
			Nick:   n.Nick,
			Bounds: &pb.Bounds{Start: int64(n.Pos()), End: int64(n.End())},
		})
	case *parser.Tag:
		e.Tags = append(e.Tags, &pb.Tag{
			Name:   n.Name,
			Bounds: &pb.Bounds{Start: int64(n.Pos()), End: int64(n.End())},
		})
	case *parser.Span:
		addEntitiesFromSpan(e, n)
	}
}
