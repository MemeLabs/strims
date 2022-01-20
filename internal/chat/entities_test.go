package chat

import (
	"testing"

	parser "github.com/MemeLabs/chat-parser"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/stretchr/testify/assert"
	"mvdan.cc/xurls/v2"
)

func TestParse(t *testing.T) {
	type entityTest struct {
		name     string
		input    string
		entities *chatv1.Message_Entities
	}

	var cases = []entityTest{
		{
			name:     "basic",
			input:    "aaaaaaaaaa",
			entities: &chatv1.Message_Entities{},
		},
		{
			name:  "emote",
			input: "test PEPE test",
			entities: &chatv1.Message_Entities{
				Emotes: []*chatv1.Message_Entities_Emote{{Name: "PEPE", Bounds: &chatv1.Message_Entities_Bounds{Start: 5, End: 9}}},
			},
		},
		{
			name:  "link",
			input: "strims.gg",
			entities: &chatv1.Message_Entities{
				Links: []*chatv1.Message_Entities_Link{{Url: "https://strims.gg", Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 9}}},
			},
		},
		{
			name:  "link",
			input: "http://strims.gg",
			entities: &chatv1.Message_Entities{
				Links: []*chatv1.Message_Entities_Link{{Url: "http://strims.gg", Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 16}}},
			},
		},
		{
			name:  "spoiler",
			input: "spoiler ||dumbledore was gay all along||",
			entities: &chatv1.Message_Entities{
				Spoilers: []*chatv1.Message_Entities_Spoiler{{Bounds: &chatv1.Message_Entities_Bounds{Start: 8, End: 40}}},
			},
		},
		{
			name:  "greentext",
			input: ">implying greentext doesn't work",
			entities: &chatv1.Message_Entities{
				GreenText: &chatv1.Message_Entities_GenericEntity{Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 32}},
			},
		},
		{
			name:  "self",
			input: "/me dies",
			entities: &chatv1.Message_Entities{
				SelfMessage: &chatv1.Message_Entities_GenericEntity{Bounds: &chatv1.Message_Entities_Bounds{Start: 4, End: 8}},
			},
		},
		{
			name:  "tag",
			input: "nsfw loud weeb nsfl google.com",
			entities: &chatv1.Message_Entities{
				Links: []*chatv1.Message_Entities_Link{{Url: "https://google.com", Bounds: &chatv1.Message_Entities_Bounds{Start: 20, End: 30}}},
				Tags: []*chatv1.Message_Entities_Tag{
					{Name: "nsfw", Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 4}},
					{Name: "loud", Bounds: &chatv1.Message_Entities_Bounds{Start: 5, End: 9}},
					{Name: "weeb", Bounds: &chatv1.Message_Entities_Bounds{Start: 10, End: 14}},
					{Name: "nsfl", Bounds: &chatv1.Message_Entities_Bounds{Start: 15, End: 19}},
				},
			},
		},
		{
			name:  "code",
			input: "`hacker mode activated`",
			entities: &chatv1.Message_Entities{
				CodeBlocks: []*chatv1.Message_Entities_CodeBlock{{Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 23}}},
			},
		},
		{
			// should not trigger weeb tag
			name:  "entity in link",
			input: "strims.gg/weeb",
			entities: &chatv1.Message_Entities{
				Links: []*chatv1.Message_Entities_Link{{Url: "https://strims.gg/weeb", Bounds: &chatv1.Message_Entities_Bounds{Start: 0, End: 14}}},
			},
		},
	}

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
	assert.Equal(t, &chatv1.Message_Entities_Bounds{Start: 4, End: 7}, result.Nicks[0].Bounds)

	extractor.RemoveNick("bob")
	result = extractor.Extract(message)
	assert.Empty(t, result.Nicks)
}

func xtractor() *entityExtractor {
	return &entityExtractor{
		parserCtx: parser.NewParserContext(parser.ParserContextValues{
			Emotes:         []string{"INFESTOR", "FIDGETLOL", "Hhhehhehe", "GameOfThrows", "Abathur", "LUL", "SURPRISE", "NoTears", "OverRustle", "DuckerZ", "Kappa", "Klappa", "DappaKappa", "BibleThump", "AngelThump", "BasedGod", "OhKrappa", "SoDoge", "WhoahDude", "MotherFuckinGame", "DaFeels", "UWOTM8", "DatGeoff", "FerretLOL", "Sippy", "Nappa", "DAFUK", "HEADSHOT", "DANKMEMES", "MLADY", "MASTERB8", "NOTMYTEMPO", "LeRuse", "YEE", "SWEATY", "PEPE", "SpookerZ", "WEEWOO", "ASLAN", "TRUMPED", "BASEDWATM8", "BERN", "Hmmm", "PepoThink", "FeelsAmazingMan", "FeelsBadMan", "FeelsGoodMan", "OhMyDog", "Wowee", "haHAA", "POTATO", "NOBULLY", "gachiGASM", "REE", "monkaS", "RaveDoge", "CuckCrab", "MiyanoHype", "ECH", "NiceMeMe", "ITSRAWWW", "Riperino", "4Head", "BabyRage", "Kreygasm", "SMOrc", "NotLikeThis", "POGGERS", "AYAYA", "PepOk", "PepoComfy", "PepoWant", "PepeHands", "BOGGED", "ComfyApe", "ApeHands", "OMEGALUL", "COGGERS", "PepoWant", "Clap", "FeelsWeirdMan", "monkaMEGA", "ComfyDog", "GIMI", "MOOBERS", "PepoBan", "ComfyAYA", "ComfyFerret", "BOOMER", "ZOOMER", "SOY", "FeelsPepoMan", "ComfyCat", "ComfyPOTATO", "SUGOI", "DJPepo", "CampFire", "ComfyYEE", "weSmart", "PepoG", "OBJECTION", "ComfyWeird", "umaruCry", "OsKrappa", "monkaHmm", "PepoHmm", "PepeComfy", "SUGOwO", "EZ", "Pepega", "shyLurk", "FeelsOkayMan", "POKE", "PepoDance", "ORDAH", "SPY", "PepoGood", "PepeJam", "LAG", "billyWeird", "SOTRIGGERED", "OnlyPretending", "cmonBruh", "VroomVroom", "mikuDance", "WAG", "PepoFight", "NeneLaugh", "PepeLaugh", "PeepoS", "SLEEPY", "GODMAN", "NOM", "FeelsDumbMan", "SEMPAI", "OSTRIGGERED", "MiyanoBird", "KING", "PIKOHH", "PepoPirate", "PepeMods", "OhISee", "WeirdChamp", "RedCard", "illyaTriggered", "SadBenis", "PeepoHappy", "ComfyWAG", "MiyanoComfy", "sataniaLUL", "DELUSIONAL", "GREED", "AYAWeird", "FeelsCountryMan", "SNAP", "PeepoRiot", "HiHi", "ComfyFeels", "MiyanoSip", "PeepoWeird", "JimFace", "HACKER", "monkaVirus", "DOUBT", "KEKW", "SHOCK", "DOIT", "GODWOMAN", "POGGIES", "SHRUG", "POGOI", "PepoSleep"},
			Nicks:          []string{},
			Tags:           []string{"nsfw", "weeb", "nsfl", "loud"},
			EmoteModifiers: []string{"mirror", "flip", "rain", "snow", "rustle", "worth", "love", "spin", "wide", "lag", "hyper"},
		}),
		urls: xurls.Relaxed(),
	}
}
