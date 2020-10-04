package chat

import (
	"testing"

	parser "github.com/MemeLabs/chat-parser"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/stretchr/testify/assert"
	"mvdan.cc/xurls/v2"
)

type entityTest struct {
	name     string
	input    string
	entities *pb.MessageEntities
}

var cases = []entityTest{
	{
		name:     "basic",
		input:    "aaaaaaaaaa",
		entities: &pb.MessageEntities{},
	},
	{
		name:  "emote",
		input: "test PEPE test",
		entities: &pb.MessageEntities{
			Emotes: []*pb.MessageEntities_Emote{{Name: "PEPE", Bounds: &pb.MessageEntities_Bounds{Start: 5, End: 9}}},
		},
	},
	{
		name:  "link",
		input: "strims.gg",
		entities: &pb.MessageEntities{
			Links: []*pb.MessageEntities_Link{{Url: "strims.gg", Bounds: &pb.MessageEntities_Bounds{Start: 0, End: 9}}},
		},
	},
	{
		name:  "spoiler",
		input: "spoiler ||dumbledore was gay all along||",
		entities: &pb.MessageEntities{
			Spoilers: []*pb.MessageEntities_Spoiler{{Bounds: &pb.MessageEntities_Bounds{Start: 8, End: 40}}},
		},
	},
	{
		name:  "greentext",
		input: ">implying greentext doesn't work",
		entities: &pb.MessageEntities{
			GreenText: &pb.MessageEntities_GenericEntity{Bounds: &pb.MessageEntities_Bounds{Start: 0, End: 32}},
		},
	},
	{
		name:  "self",
		input: "/me dies",
		entities: &pb.MessageEntities{
			SelfMessage: &pb.MessageEntities_GenericEntity{Bounds: &pb.MessageEntities_Bounds{Start: 4, End: 8}},
		},
	},
	{
		name:  "tag",
		input: "nsfw loud weeb nsfl google.com",
		entities: &pb.MessageEntities{
			Links: []*pb.MessageEntities_Link{{Url: "google.com", Bounds: &pb.MessageEntities_Bounds{Start: 20, End: 30}}},
			Tags: []*pb.MessageEntities_Tag{
				{Name: "nsfw", Bounds: &pb.MessageEntities_Bounds{Start: 0, End: 4}},
				{Name: "loud", Bounds: &pb.MessageEntities_Bounds{Start: 5, End: 9}},
				{Name: "weeb", Bounds: &pb.MessageEntities_Bounds{Start: 10, End: 14}},
				{Name: "nsfl", Bounds: &pb.MessageEntities_Bounds{Start: 15, End: 19}},
			},
		},
	},
	{
		name:  "code",
		input: "`hacker mode activated`",
		entities: &pb.MessageEntities{
			CodeBlocks: []*pb.MessageEntities_CodeBlock{{Bounds: &pb.MessageEntities_Bounds{Start: 0, End: 23}}},
		},
	},
	{
		// should not trigger weeb tag
		name:  "entity in link",
		input: "strims.gg/weeb",
		entities: &pb.MessageEntities{
			Links: []*pb.MessageEntities_Link{{Url: "strims.gg/weeb", Bounds: &pb.MessageEntities_Bounds{Start: 0, End: 14}}},
		},
	},
}

func TestParse(t *testing.T) {
	extractor := xtractor()
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			t.Parallel()
			result := extractor.Extract(c.input)
			assert.Equal(t, c.entities.CodeBlocks, result.CodeBlocks)
			assert.Equal(t, c.entities.Emotes, result.Emotes)
			assert.Equal(t, c.entities.Tags, result.Tags)
			assert.Equal(t, c.entities.Spoilers, result.Spoilers)
			assert.Equal(t, c.entities.Links, result.Links)
			assert.Equal(t, c.entities.GreenText, result.GreenText)
			assert.Equal(t, c.entities.Nicks, result.Nicks)
			assert.Equal(t, c.entities.SelfMessage, result.SelfMessage)
		})
	}
}

func TestNicks(t *testing.T) {
	extractor := xtractor()

	message := "hey bob how are you"

	result := extractor.Extract(message)
	assert.Empty(t, result.Nicks)
	extractor.AddNick("bob")

	result = extractor.Extract(message)
	assert.Equal(t, 1, len(result.Nicks))
	assert.Equal(t, "bob", result.Nicks[0].Nick)
	assert.Equal(t, &pb.MessageEntities_Bounds{Start: 4, End: 7}, result.Nicks[0].Bounds)

	extractor.RemoveNick("bob")
	result = extractor.Extract(message)
	assert.Empty(t, result.Nicks)
}

func xtractor() *EntityExtractor {
	return &EntityExtractor{
		parserCtx: parser.NewParserContext(parser.ParserContextValues{
			Emotes:         []string{"INFESTOR", "FIDGETLOL", "Hhhehhehe", "GameOfThrows", "Abathur", "LUL", "SURPRISE", "NoTears", "OverRustle", "DuckerZ", "Kappa", "Klappa", "DappaKappa", "BibleThump", "AngelThump", "BasedGod", "OhKrappa", "SoDoge", "WhoahDude", "MotherFuckinGame", "DaFeels", "UWOTM8", "DatGeoff", "FerretLOL", "Sippy", "Nappa", "DAFUK", "HEADSHOT", "DANKMEMES", "MLADY", "MASTERB8", "NOTMYTEMPO", "LeRuse", "YEE", "SWEATY", "PEPE", "SpookerZ", "WEEWOO", "ASLAN", "TRUMPED", "BASEDWATM8", "BERN", "Hmmm", "PepoThink", "FeelsAmazingMan", "FeelsBadMan", "FeelsGoodMan", "OhMyDog", "Wowee", "haHAA", "POTATO", "NOBULLY", "gachiGASM", "REE", "monkaS", "RaveDoge", "CuckCrab", "MiyanoHype", "ECH", "NiceMeMe", "ITSRAWWW", "Riperino", "4Head", "BabyRage", "Kreygasm", "SMOrc", "NotLikeThis", "POGGERS", "AYAYA", "PepOk", "PepoComfy", "PepoWant", "PepeHands", "BOGGED", "ComfyApe", "ApeHands", "OMEGALUL", "COGGERS", "PepoWant", "Clap", "FeelsWeirdMan", "monkaMEGA", "ComfyDog", "GIMI", "MOOBERS", "PepoBan", "ComfyAYA", "ComfyFerret", "BOOMER", "ZOOMER", "SOY", "FeelsPepoMan", "ComfyCat", "ComfyPOTATO", "SUGOI", "DJPepo", "CampFire", "ComfyYEE", "weSmart", "PepoG", "OBJECTION", "ComfyWeird", "umaruCry", "OsKrappa", "monkaHmm", "PepoHmm", "PepeComfy", "SUGOwO", "EZ", "Pepega", "shyLurk", "FeelsOkayMan", "POKE", "PepoDance", "ORDAH", "SPY", "PepoGood", "PepeJam", "LAG", "billyWeird", "SOTRIGGERED", "OnlyPretending", "cmonBruh", "VroomVroom", "mikuDance", "WAG", "PepoFight", "NeneLaugh", "PepeLaugh", "PeepoS", "SLEEPY", "GODMAN", "NOM", "FeelsDumbMan", "SEMPAI", "OSTRIGGERED", "MiyanoBird", "KING", "PIKOHH", "PepoPirate", "PepeMods", "OhISee", "WeirdChamp", "RedCard", "illyaTriggered", "SadBenis", "PeepoHappy", "ComfyWAG", "MiyanoComfy", "sataniaLUL", "DELUSIONAL", "GREED", "AYAWeird", "FeelsCountryMan", "SNAP", "PeepoRiot", "HiHi", "ComfyFeels", "MiyanoSip", "PeepoWeird", "JimFace", "HACKER", "monkaVirus", "DOUBT", "KEKW", "SHOCK", "DOIT", "GODWOMAN", "POGGIES", "SHRUG", "POGOI", "PepoSleep"},
			Nicks:          []string{},
			Tags:           []string{"nsfw", "weeb", "nsfl", "loud"},
			EmoteModifiers: []string{"mirror", "flip", "rain", "snow", "rustle", "worth", "love", "spin", "wide", "lag", "hyper"},
		}),
		urls: xurls.Relaxed(),
	}
}
