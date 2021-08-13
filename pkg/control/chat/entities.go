package chat

import (
	"math/rand"
	"regexp"

	parser "github.com/MemeLabs/chat-parser"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
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
	rareRate  float64
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
func (x *EntityExtractor) Extract(msg string) *chatv1.Message_Entities {
	e := &chatv1.Message_Entities{}

	for _, b := range x.urls.FindAllStringIndex(msg, -1) {
		e.Links = append(e.Links, &chatv1.Message_Entities_Link{
			Url:    msg[b[0]:b[1]],
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
